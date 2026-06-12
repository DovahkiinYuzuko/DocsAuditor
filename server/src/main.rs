use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::path::{Path, PathBuf};
use url::Url;

mod parser;
mod state;
mod auditor;
mod locker;

use state::hfsm::{Hfsm, Event};
use auditor::{audit_symbols, AuditIssue, AuditIssueType};

struct Backend {
    client: Client,
    state: Arc<Mutex<Hfsm>>,
    root_path: Arc<Mutex<Option<PathBuf>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut hfsm = self.state.lock().await;
        hfsm.dispatch(Event::Initialize);

        let mut root_path = self.root_path.lock().await;
        if let Some(uri) = params.root_uri {
            if let Ok(path) = uri.to_file_path() {
                *root_path = Some(path);
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Docs Auditor LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        let mut hfsm = self.state.lock().await;
        hfsm.dispatch(Event::Shutdown);
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.on_change(params.text_document.uri, change.text.clone()).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.on_change(params.text_document.uri, text).await;
        } else {
            if let Ok(path) = params.text_document.uri.to_file_path() {
                if let Ok(text) = tokio::fs::read_to_string(&path).await {
                    self.on_change(params.text_document.uri, text).await;
                }
            }
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let mut actions = Vec::new();

        for diagnostic in params.context.diagnostics {
            if let Some(data) = &diagnostic.data {
                if let Ok(issue) = serde_json::from_value::<AuditIssue>(data.clone()) {
                    if issue.issue_type == AuditIssueType::LineNumberMissing || issue.issue_type == AuditIssueType::LineNumberMismatch {
                        if let Some(code_range) = issue.code_line_range {
                            let new_text = format!(" (L{}-{})", code_range.0, code_range.1);
                            
                            let edit = TextEdit {
                                range: Range {
                                    start: Position {
                                        line: diagnostic.range.end.line,
                                        character: diagnostic.range.end.character,
                                    },
                                    end: Position {
                                        line: diagnostic.range.end.line,
                                        character: diagnostic.range.end.character,
                                    },
                                },
                                new_text,
                            };

                            let mut changes = std::collections::HashMap::new();
                            changes.insert(uri.clone(), vec![edit]);

                            let action = CodeAction {
                                title: format!(
                                    "行番号 (L{}-{}) を仕様書に自動追記する",
                                    code_range.0, code_range.1
                                ),
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: Some(vec![diagnostic.clone()]),
                                edit: Some(WorkspaceEdit {
                                    changes: Some(changes),
                                    document_changes: None,
                                    change_annotations: None,
                                }),
                                is_preferred: Some(true),
                                disabled: None,
                                data: None,
                                command: None,
                            };

                            actions.push(CodeActionOrCommand::CodeAction(action));
                        }
                    }
                }
            }
        }

        Ok(Some(actions))
    }
}

impl Backend {
    async fn on_change(&self, uri: Url, text: String) {
        {
            self.state.lock().await.dispatch(Event::DocumentChanged);
        }

        let root_path_opt = { self.root_path.lock().await.clone() };
        let root_path = match root_path_opt {
            Some(path) => path,
            None => {
                self.state.lock().await.dispatch(Event::AnalysisCompleted);
                return;
            }
        };

        let file_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                self.state.lock().await.dispatch(Event::AnalysisCompleted);
                return;
            }
        };

        let file_path_str = file_path.to_string_lossy();
        let is_spec = file_path_str.contains("docs/variables'n'functions") || 
                      file_path_str.contains("docs\\variables'n'functions");

        let mut spec_uri = uri.clone();
        let mut spec_text = text.clone();
        let mut code_text = String::new();
        let mut matched = false;

        if is_spec {
            if let Some(file_name) = file_path.file_name().and_then(|f| f.to_str()) {
                if file_name.starts_with('[') {
                    if let Some(end_bracket) = file_name.find(']') {
                        let lang = &file_name[1..end_bracket];
                        let name_without_lang = &file_name[end_bracket + 1..file_name.len() - 3];
                        
                        let extension = match lang.to_lowercase().as_str() {
                            "rust" => "rs",
                            "typescript" => "ts",
                            "javascript" => "js",
                            "python" => "py",
                            _ => "",
                        };

                        if !extension.is_empty() {
                            let target_filename = format!("{}.{}", name_without_lang, extension);
                            if let Some(found_code_path) = find_file_in_dir(&root_path, &target_filename).await {
                                if let Ok(content) = tokio::fs::read_to_string(&found_code_path).await {
                                    code_text = content;
                                    matched = true;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            if file_path.file_name().is_some() {
                let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let lang_prefix = match extension {
                    "rs" => "Rust",
                    "ts" => "TypeScript",
                    "js" => "JavaScript",
                    "py" => "Python",
                    _ => "",
                };

                if !lang_prefix.is_empty() {
                    let spec_filename = format!("[{}]{}.md", lang_prefix, stem);
                    let spec_dir = root_path.join("docs").join("variables'n'functions");
                    let spec_file_path = spec_dir.join(&spec_filename);

                    if spec_file_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&spec_file_path).await {
                            spec_text = content;
                            code_text = text.clone();
                            if let Ok(s_uri) = Url::from_file_path(&spec_file_path) {
                                spec_uri = s_uri;
                                matched = true;
                            }
                        }
                    }
                }
            }
        }

        if !matched {
            self.state.lock().await.dispatch(Event::AnalysisCompleted);
            return;
        }

        let spec_symbols = parser::parse_markdown_spec(&spec_text);
        let code_symbols = parser::parse_rust_code(&code_text);

        let issues = audit_symbols(&spec_symbols, &code_symbols);

        let mut diagnostics = Vec::new();
        let mut line_issues = Vec::new();

        for issue in &issues {
            let line_num = issue.spec_line.saturating_sub(1) as u32;
            let line_content = spec_text.lines().nth(line_num as usize).unwrap_or("");
            let start_char = line_content.find(|c: char| !c.is_whitespace()).unwrap_or(0) as u32;
            let end_char = line_content.len() as u32;

            let severity = match issue.issue_type {
                AuditIssueType::MissingInCode => DiagnosticSeverity::ERROR,
                AuditIssueType::TypeMismatch | AuditIssueType::ParamCountMismatch | AuditIssueType::ReturnTypeMismatch => DiagnosticSeverity::WARNING,
                AuditIssueType::LineNumberMissing | AuditIssueType::LineNumberMismatch => {
                    line_issues.push(issue.clone());
                    DiagnosticSeverity::HINT
                }
            };

            let diagnostic_data = serde_json::to_value(issue).ok();

            let d = Diagnostic {
                range: Range {
                    start: Position { line: line_num, character: start_char },
                    end: Position { line: line_num, character: end_char },
                },
                severity: Some(severity),
                code: None,
                code_description: None,
                source: Some("Docs Auditor".to_string()),
                message: issue.message.clone(),
                related_information: None,
                tags: None,
                data: diagnostic_data,
            };

            diagnostics.push(d);
        }

        self.client.publish_diagnostics(spec_uri.clone(), diagnostics, None).await;

        let spec_path_opt = spec_uri.to_file_path().ok();

        // 4. レポートファイルの自動生成・削除
        let report_path = root_path.join("variables_functions_audit_report.md");
        if issues.is_empty() {
            if report_path.exists() {
                let _ = tokio::fs::remove_file(&report_path).await;
            }
        } else if let Some(ref spec_path) = spec_path_opt {
            let mut report_content = String::new();
            report_content.push_str("# 整合性監査レポート (Docs Auditor)\n\n");
            report_content.push_str("仕様書とコードの整合性検査で以下の不一致が検出されました。各項目を修正してください。\n\n");
            report_content.push_str("## 不一致項目 (TODO)\n\n");

            for issue in &issues {
                let issue_type_str = match issue.issue_type {
                    AuditIssueType::MissingInCode => "コード側定義なし",
                    AuditIssueType::TypeMismatch => "型ミスマッチ",
                    AuditIssueType::ParamCountMismatch => "引数個数ミスマッチ",
                    AuditIssueType::ReturnTypeMismatch => "戻り値ミスマッチ",
                    AuditIssueType::LineNumberMissing => "行番号未記載",
                    AuditIssueType::LineNumberMismatch => "行番号ミスマッチ",
                };

                let spec_relative = spec_path.strip_prefix(&root_path)
                    .unwrap_or(spec_path)
                    .to_string_lossy();

                report_content.push_str(&format!(
                    "- [ ] **{}** (シンボル: `{}`)\n  - **内容**: {}\n  - **仕様書箇所**: [{}](file:///{}) L{}\n",
                    issue_type_str,
                    issue.name,
                    issue.message,
                    spec_relative,
                    spec_path.to_string_lossy().replace('\\', "/"),
                    issue.spec_line
                ));
            }

            let _ = tokio::fs::write(&report_path, report_content).await;
        }

        // 5. 設定の問い合わせ (autoInjection の確認)
        let config_item = ConfigurationItem {
            scope_uri: Some(spec_uri.clone()),
            section: Some("docsAuditor.autoInjection".to_string()),
        };
        let mut auto_injection = false;
        if let Ok(configs) = self.client.configuration(vec![config_item]).await {
            if let Some(val) = configs.first() {
                auto_injection = val.as_bool().unwrap_or(false);
            }
        }

        if auto_injection && !line_issues.is_empty() {
            self.state.lock().await.dispatch(Event::TriggerAutoInjection);

            if let Some(spec_path) = spec_path_opt {
                if let Some(_lock) = locker::FileLocker::try_lock(&spec_path) {
                    self.state.lock().await.dispatch(Event::LockAcquired);

                    let mut updated_lines: Vec<String> = spec_text.lines().map(|s| s.to_string()).collect();

                    for issue in line_issues {
                        let line_idx = issue.spec_line.saturating_sub(1);
                        if line_idx < updated_lines.len() {
                            let line = &updated_lines[line_idx];
                            if let Some(code_range) = issue.code_line_range {
                                let line_regex = regex::Regex::new(r"\s*\(L\d+(?:-\d+)?\)\s*$").unwrap();
                                let clean_line = line_regex.replace(line, "").trim_end().to_string();
                                let updated_line = format!("{} (L{}-{})", clean_line, code_range.0, code_range.1);
                                updated_lines[line_idx] = updated_line;
                            }
                        }
                    }

                    let new_spec_content = updated_lines.join("\n");
                    if tokio::fs::write(&spec_path, new_spec_content).await.is_ok() {
                        self.state.lock().await.dispatch(Event::WriteCompleted);
                        self.state.lock().await.dispatch(Event::LockReleased);
                    } else {
                        self.state.lock().await.dispatch(Event::WriteError);
                        self.state.lock().await.dispatch(Event::RecoveryCompleted);
                    }
                } else {
                    self.state.lock().await.dispatch(Event::LockFailed);
                    self.state.lock().await.dispatch(Event::RecoveryCompleted);
                }
            }
        } else {
            self.state.lock().await.dispatch(Event::AnalysisCompleted);
        }
    }
}

async fn find_file_in_dir(dir: &Path, filename: &str) -> Option<PathBuf> {
    let mut read_dir = match tokio::fs::read_dir(dir).await {
        Ok(d) => d,
        Err(_) => return None,
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "target" || dir_name == "node_modules" || dir_name == "docs" || dir_name == ".git" {
                continue;
            }
            if let Some(found) = Box::pin(find_file_in_dir(&path, filename)).await {
                return Some(found);
            }
        } else if path.is_file() {
            if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
                return Some(path);
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend {
        client,
        state: Arc::new(Mutex::new(Hfsm::new())),
        root_path: Arc::new(Mutex::new(None)),
    });

    Server::new(stdin, stdout, messages).serve(service).await;
}
