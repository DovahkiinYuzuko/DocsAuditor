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
mod i18n;

use state::hfsm::{Hfsm, Event};
use auditor::{audit_symbols, AuditIssue, AuditIssueType};
use i18n::{get_message, MessageKey};

#[derive(Clone)]
struct Backend {
    client: Client,
    state: Arc<Mutex<Hfsm>>,
    root_path: Arc<Mutex<Option<PathBuf>>>,
    locale: Arc<Mutex<String>>,
    issues_cache: Arc<Mutex<std::collections::HashMap<Url, Vec<AuditIssue>>>>,
    project_used_symbols: Arc<Mutex<Option<std::collections::HashSet<String>>>>,
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

        // initialization_options から locale を抽出して保存する
        if let Some(options) = params.initialization_options {
            if let Some(locale_val) = options.get("locale") {
                if let Some(locale_str) = locale_val.as_str() {
                    let mut locale = self.locale.lock().await;
                    *locale = locale_str.to_string();
                }
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
        
        let this = self.clone();
        tokio::spawn(async move {
            this.run_initial_scan().await;
        });
    }

    async fn shutdown(&self) -> Result<()> {
        let mut hfsm = self.state.lock().await;
        hfsm.dispatch(Event::Shutdown);
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, params.text_document.text, true).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.on_change(params.text_document.uri, change.text.clone(), false).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.on_change(params.text_document.uri, text, true).await;
        } else {
            if let Ok(path) = params.text_document.uri.to_file_path() {
                if let Ok(text) = tokio::fs::read_to_string(&path).await {
                    self.on_change(params.text_document.uri, text, true).await;
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
                            let mut text_edit = None;
                            if let Ok(path) = uri.to_file_path() {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let line_num = diagnostic.range.start.line as usize;
                                    if let Some(line_content) = content.lines().nth(line_num) {
                                        let line_regex = regex::Regex::new(r"\s*\(L\d+(?:-\d+)?\)\s*$").unwrap();
                                        let clean_line = line_regex.replace(line_content, "").trim_end().to_string();
                                        let new_line_content = format!("{} (L{}-{})", clean_line, code_range.0, code_range.1);

                                        text_edit = Some(TextEdit {
                                            range: Range {
                                                start: Position {
                                                    line: line_num as u32,
                                                    character: 0,
                                                },
                                                end: Position {
                                                    line: line_num as u32,
                                                    character: line_content.len() as u32,
                                                },
                                            },
                                            new_text: new_line_content,
                                        });
                                    }
                                }
                            }

                            if let Some(edit) = text_edit {
                                let mut changes = std::collections::HashMap::new();
                                changes.insert(uri.clone(), vec![edit]);

                                let locale = self.locale.lock().await;
                                let title = get_message(
                                    &MessageKey::CodeActionTitle(format!("L{}-{}", code_range.0, code_range.1)),
                                    &locale,
                                );

                                let action = CodeAction {
                                    title,
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
        }

        Ok(Some(actions))
    }
}

impl Backend {
    async fn on_change(&self, uri: Url, text: String, force_update_cache: bool) {
        let _: &tokio::sync::Mutex<crate::state::hfsm::Hfsm> = &self.state;
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
        let mut target_lang = String::new();

        if is_spec {
            if let Some(file_name) = file_path.file_name().and_then(|f| f.to_str()) {
                if file_name.starts_with('[') {
                    if let Some(end_bracket) = file_name.find(']') {
                        let lang = &file_name[1..end_bracket];
                        target_lang = lang.to_lowercase();
                        
                        // DoS 対策: 長さと拡張子を確認した上で安全にスライス
                        if file_name.ends_with(".md") && file_name.len() >= end_bracket + 4 {
                            let name_without_lang = &file_name[end_bracket + 1..file_name.len() - 3];
                            
                            let extension = match target_lang.as_str() {
                                "rust" => "rs",
                                "typescript" => "ts",
                                "javascript" => "js",
                                "python" => "py",
                                "go" => "go",
                                "c" => "c",
                                "cpp" => "cpp",
                                "csharp" => "cs",
                                "ruby" => "rb",
                                "swift" => "swift",
                                "kotlin" => "kt",
                                "java" => "java",
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
            }
        } else {
            if file_path.file_name().is_some() {
                let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                target_lang = match extension {
                    "rs" => "rust".to_string(),
                    "ts" => "typescript".to_string(),
                    "js" => "javascript".to_string(),
                    "py" => "python".to_string(),
                    "go" => "go".to_string(),
                    "c" | "h" => "c".to_string(),
                    "cpp" | "hpp" | "cc" | "cxx" => "cpp".to_string(),
                    "cs" => "csharp".to_string(),
                    "rb" => "ruby".to_string(),
                    "swift" => "swift".to_string(),
                    "kt" | "kts" => "kotlin".to_string(),
                    "java" => "java".to_string(),
                    _ => "".to_string(),
                };
                let lang_prefix = match extension {
                    "rs" => "Rust",
                    "ts" => "TypeScript",
                    "js" => "JavaScript",
                    "py" => "Python",
                    "go" => "Go",
                    "c" | "h" => "C",
                    "cpp" | "hpp" | "cc" | "cxx" => "CPP",
                    "cs" => "CSharp",
                    "rb" => "Ruby",
                    "swift" => "Swift",
                    "kt" | "kts" => "Kotlin",
                    "java" => "Java",
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
        let code_symbols = parser::parse_code(&code_text, &target_lang);
        
        // パフォーマンス最適化: キャッシュを使用してDidChange時の全体スキャンを削減
        let project_used = {
            let mut cache = self.project_used_symbols.lock().await;
            if force_update_cache || cache.is_none() {
                let symbols = collect_project_used_symbols(&root_path).await;
                *cache = Some(symbols.clone());
                symbols
            } else {
                cache.as_ref().unwrap().clone()
            }
        };

        if force_update_cache {
            self.run_scan(Some(project_used)).await;
            return;
        }
        let spec_path_opt = spec_uri.to_file_path().ok();

        let locale = self.locale.lock().await;
        let issues = audit_symbols(&spec_symbols, &code_symbols, &project_used, &locale);

        let mut diagnostics = Vec::new();
        let mut line_issues = Vec::new();

        for issue in &issues {
            let line_num = issue.spec_line.saturating_sub(1) as u32;
            let line_content = spec_text.lines().nth(line_num as usize).unwrap_or("");
            let start_char = line_content.find(|c: char| !c.is_whitespace()).unwrap_or(0) as u32;
            let end_char = line_content.len() as u32;

            let severity = match issue.issue_type {
                AuditIssueType::MissingInCode => DiagnosticSeverity::ERROR,
                AuditIssueType::TypeMismatch | AuditIssueType::ParamCountMismatch | AuditIssueType::ReturnTypeMismatch | AuditIssueType::DependencyNotUsed | AuditIssueType::DeadCode => DiagnosticSeverity::WARNING,
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

        self.client.publish_diagnostics(normalize_url(&spec_uri), diagnostics, None).await;

        // issues_cache を更新
        {
            let mut cache = self.issues_cache.lock().await;
            cache.insert(normalize_url(&spec_uri), issues.clone());
        }

        // 全てのエラーをキャッシュから集計
        let mut all_issues = Vec::new();
        {
            let cache = self.issues_cache.lock().await;
            for (s_uri, s_issues) in cache.iter() {
                for issue in s_issues {
                    all_issues.push((s_uri.clone(), issue.clone()));
                }
            }
        }

        // 4. レポートファイルの自動生成・削除
        let report_path = root_path.join("variables_functions_audit_report.md");
        if all_issues.is_empty() {
            if report_path.exists() {
                let _ = tokio::fs::remove_file(&report_path).await;
            }
        } else {
            let mut report_content = String::new();
            report_content.push_str(&get_message(&MessageKey::ReportTitle, &locale));
            report_content.push_str(&get_message(&MessageKey::ReportHeader, &locale));
            report_content.push_str(&get_message(&MessageKey::ReportSectionTitle, &locale));

            let loc = locale.to_lowercase();
            let is_ja = loc.starts_with("ja");
            let is_zh_cn = loc.starts_with("zh-cn") || loc.starts_with("zh-hans");
            let is_zh_tw = loc.starts_with("zh-tw") || loc.starts_with("zh-hk") || loc.starts_with("zh-hant");
            let is_ko = loc.starts_with("ko");
            let is_et = loc.starts_with("et");
            let is_vi = loc.starts_with("vi");
            let is_es = loc.starts_with("es");
            let is_fr = loc.starts_with("fr");
            let is_de = loc.starts_with("de");

            for (s_uri, issue) in &all_issues {
                let issue_type_str = match issue.issue_type {
                    AuditIssueType::MissingInCode => {
                        if is_ja { "コード側定義なし" }
                        else if is_zh_cn { "代码侧未定义" }
                        else if is_zh_tw { "程式碼側未定義" }
                        else if is_ko { "코드 측 정의 없음" }
                        else if is_et { "Koodis puudub definitsioon" }
                        else if is_vi { "Thiếu định nghĩa trong mã" }
                        else if is_es { "Falta definición en código" }
                        else if is_fr { "Définition manquante dans le code" }
                        else if is_de { "Fehlende Definition im Code" }
                        else { "Missing in Code" }
                    }
                    AuditIssueType::TypeMismatch => {
                        if is_ja { "型ミスマッチ" }
                        else if is_zh_cn { "类型不匹配" }
                        else if is_zh_tw { "型態不匹配" }
                        else if is_ko { "타입 불일치" }
                        else if is_et { "Tüübi lahknevus" }
                        else if is_vi { "Sai kiểu dữ liệu" }
                        else if is_es { "Discrepancia de tipo" }
                        else if is_fr { "Incompatibilité de type" }
                        else if is_de { "Typkonflikt" }
                        else { "Type Mismatch" }
                    }
                    AuditIssueType::ParamCountMismatch => {
                        if is_ja { "引数個数ミスマッチ" }
                        else if is_zh_cn { "参数数量不匹配" }
                        else if is_zh_tw { "參數數量不匹配" }
                        else if is_ko { "매개변수 개수 불일치" }
                        else if is_et { "Parameetrite arvu lahknevus" }
                        else if is_vi { "Số lượng tham số không khớp" }
                        else if is_es { "Discrepancia de parámetros" }
                        else if is_fr { "Incompatibilité du nombre de paramètres" }
                        else if is_de { "Parameteranzahl-Konflikt" }
                        else { "Parameter Count Mismatch" }
                    }
                    AuditIssueType::ReturnTypeMismatch => {
                        if is_ja { "戻り値ミスマッチ" }
                        else if is_zh_cn { "返回值类型不匹配" }
                        else if is_zh_tw { "傳回值型態不匹配" }
                        else if is_ko { "반환 타입 불일치" }
                        else if is_et { "Tagastustüübi lahknevus" }
                        else if is_vi { "Kiểu trả về không khớp" }
                        else if is_es { "Discrepancia de tipo de retorno" }
                        else if is_fr { "Incompatibilité du type de retour" }
                        else if is_de { "Rückgabetyp-Konflikt" }
                        else { "Return Type Mismatch" }
                    }
                    AuditIssueType::LineNumberMissing => {
                        if is_ja { "行番号未記載" }
                        else if is_zh_cn { "行号未填写" }
                        else if is_zh_tw { "行號未填寫" }
                        else if is_ko { "라인 번호 누락" }
                        else if is_et { "Reanumber puudub" }
                        else if is_vi { "Chưa ghi số dòng" }
                        else if is_es { "Falta número de línea" }
                        else if is_fr { "Numéro de ligne manquant" }
                        else if is_de { "Zeilennummer fehlt" }
                        else { "Line Number Missing" }
                    }
                    AuditIssueType::LineNumberMismatch => {
                        if is_ja { "行番号ミスマッチ" }
                        else if is_zh_cn { "行号不匹配" }
                        else if is_zh_tw { "行號不匹配" }
                        else if is_ko { "라인 번호 불일치" }
                        else if is_et { "Reanumbri lahknevus" }
                        else if is_vi { "Số dòng không khớp" }
                        else if is_es { "Discrepancia de número de línea" }
                        else if is_fr { "Incompatibilité de dynamic número de línea" } // スペルミス防ぐためオリジナルを考慮
                        else if is_de { "Zeilennummern-Konflikt" }
                        else { "Line Number Mismatch" }
                    }
                    AuditIssueType::DependencyNotUsed => {
                        if is_ja { "依存先未使用" }
                        else if is_zh_cn { "依赖项未使用" }
                        else if is_zh_tw { "依賴項未使用" }
                        else if is_ko { "의존성 미사용" }
                        else if is_et { "Kasutamata sõltuvus" }
                        else if is_vi { "Phụ thuộc chưa dùng" }
                        else if is_es { "Dependencia no utilizada" }
                        else if is_fr { "Dépendance non utilisée" }
                        else if is_de { "Abhängigkeit nicht verwendet" }
                        else { "Dependency Not Used" }
                    }
                    AuditIssueType::DeadCode => {
                        if is_ja { "デッドコード" }
                        else if is_zh_cn { "无用代码" }
                        else if is_zh_tw { "無用程式碼" }
                        else if is_ko { "데드 코드" }
                        else if is_et { "Surnud kood" }
                        else if is_vi { "Mã chết" }
                        else if is_es { "Código muerto" }
                        else if is_fr { "Code mort" }
                        else if is_de { "Toter Code" }
                        else { "Dead Code" }
                    }
                };

                let spec_path_buf = s_uri.to_file_path().unwrap_or_else(|_| PathBuf::from(""));
                let spec_relative = spec_path_buf.strip_prefix(&root_path)
                    .unwrap_or(&spec_path_buf)
                    .to_string_lossy();

                report_content.push_str(&format!(
                    "- [ ] **{}** (シンボル: `{}`)\n  - **内容**: {}\n  - **仕様書箇所**: [{}](file:///{}) L{}\n",
                    issue_type_str,
                    escape_markdown(&issue.name),
                    escape_markdown(&issue.message),
                    spec_relative,
                    spec_path_buf.to_string_lossy().replace('\\', "/"),
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

    async fn run_initial_scan(&self) {
        // クライアントの初期化完了を少し待つための安全スリープ
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.run_scan(None).await;
    }

    async fn run_scan(&self, project_used_precalculated: Option<std::collections::HashSet<String>>) {
        let root_path_opt = { self.root_path.lock().await.clone() };
        let root_path = match root_path_opt {
            Some(path) => path,
            None => return,
        };

        let spec_dir = root_path.join("docs").join("variables'n'functions");
        if !spec_dir.exists() {
            return;
        }

        let mut read_dir = match tokio::fs::read_dir(&spec_dir).await {
            Ok(d) => d,
            Err(_) => return,
        };

        let locale = { self.locale.lock().await.clone() };

        // 起動時自動インジェクション設定 of the 確認（ループの外で1回だけ行う）
        let config_item = ConfigurationItem {
            scope_uri: None,
            section: Some("docsAuditor.autoInjection".to_string()),
        };
        let mut auto_injection = false;
        if let Ok(configs) = self.client.configuration(vec![config_item]).await {
            if let Some(val) = configs.first() {
                auto_injection = val.as_bool().unwrap_or(false);
            }
        }

        // プロジェクト全体のシンボル出現情報をループの前に1度だけ収集
        let project_used = match project_used_precalculated {
            Some(symbols) => symbols,
            None => collect_project_used_symbols(&root_path).await,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename.ends_with(".md") && filename.starts_with('[') {
                    if let Ok(spec_text) = tokio::fs::read_to_string(&path).await {
                        if let Ok(spec_uri) = Url::from_file_path(&path) {
                            if let Some(end_bracket) = filename.find(']') {
                                let lang = &filename[1..end_bracket];
                                let target_lang = lang.to_lowercase();
                                let name_without_lang = &filename[end_bracket + 1..filename.len() - 3];
                                
                                let extension = match target_lang.as_str() {
                                    "rust" => "rs",
                                    "typescript" => "ts",
                                    "javascript" => "js",
                                    "python" => "py",
                                    "go" => "go",
                                    "c" => "c",
                                    "cpp" => "cpp",
                                    "csharp" => "cs",
                                    "ruby" => "rb",
                                    "swift" => "swift",
                                    "kotlin" => "kt",
                                    "java" => "java",
                                    _ => "",
                                };

                                if !extension.is_empty() {
                                    let target_filename = format!("{}.{}", name_without_lang, extension);
                                    if let Some(found_code_path) = find_file_in_dir(&root_path, &target_filename).await {
                                        if let Ok(code_text) = tokio::fs::read_to_string(&found_code_path).await {
                                            let spec_symbols = parser::parse_markdown_spec(&spec_text);
                                            let code_symbols = parser::parse_code(&code_text, &target_lang);
                                            
                                            // 事前収集した project_used を再利用
                                            let issues = audit_symbols(&spec_symbols, &code_symbols, &project_used, &locale);
                                            
                                            let mut diagnostics = Vec::new();
                                            let mut line_issues = Vec::new();
                                            for issue in &issues {
                                                let line_num = issue.spec_line.saturating_sub(1) as u32;
                                                let line_content = spec_text.lines().nth(line_num as usize).unwrap_or("");
                                                let start_char = line_content.find(|c: char| !c.is_whitespace()).unwrap_or(0) as u32;
                                                let end_char = line_content.len() as u32;

                                                let severity = match issue.issue_type {
                                                    AuditIssueType::MissingInCode => DiagnosticSeverity::ERROR,
                                                    AuditIssueType::TypeMismatch | AuditIssueType::ParamCountMismatch | AuditIssueType::ReturnTypeMismatch | AuditIssueType::DependencyNotUsed | AuditIssueType::DeadCode => DiagnosticSeverity::WARNING,
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
                                            
                                            self.client.publish_diagnostics(normalize_url(&spec_uri), diagnostics, None).await;
                                            
                                            {
                                                let mut cache = self.issues_cache.lock().await;
                                                cache.insert(normalize_url(&spec_uri), issues);
                                            }

                                            if auto_injection && !line_issues.is_empty() {
                                                if let Some(_lock) = locker::FileLocker::try_lock(&path) {
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
                                                    let _ = tokio::fs::write(&path, new_spec_content).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut all_issues = Vec::new();
        {
            let cache = self.issues_cache.lock().await;
            for (uri, issues) in cache.iter() {
                for issue in issues {
                    all_issues.push((uri.clone(), issue.clone()));
                }
            }
        }

        let report_path = root_path.join("variables_functions_audit_report.md");
        if all_issues.is_empty() {
            if report_path.exists() {
                let _ = tokio::fs::remove_file(&report_path).await;
            }
        } else {
            let mut report_content = String::new();
            report_content.push_str(&get_message(&MessageKey::ReportTitle, &locale));
            report_content.push_str(&get_message(&MessageKey::ReportHeader, &locale));
            report_content.push_str(&get_message(&MessageKey::ReportSectionTitle, &locale));

            let loc = locale.to_lowercase();
            let is_ja = loc.starts_with("ja");
            let is_zh_cn = loc.starts_with("zh-cn") || loc.starts_with("zh-hans");
            let is_zh_tw = loc.starts_with("zh-tw") || loc.starts_with("zh-hk") || loc.starts_with("zh-hant");
            let is_ko = loc.starts_with("ko");
            let is_et = loc.starts_with("et");
            let is_vi = loc.starts_with("vi");
            let is_es = loc.starts_with("es");
            let is_fr = loc.starts_with("fr");
            let is_de = loc.starts_with("de");

            for (s_uri, issue) in &all_issues {
                let issue_type_str = match issue.issue_type {
                    AuditIssueType::MissingInCode => {
                        if is_ja { "コード側定義なし" }
                        else if is_zh_cn { "代码侧未定义" }
                        else if is_zh_tw { "程式碼側未定義" }
                        else if is_ko { "코드 측 정의 없음" }
                        else if is_et { "Koodis puudub definitsioon" }
                        else if is_vi { "Thiếu định nghĩa trong mã" }
                        else if is_es { "Falta definición en código" }
                        else if is_fr { "Définition manquante dans le code" }
                        else if is_de { "Fehlende Definition im Code" }
                        else { "Missing in Code" }
                    }
                    AuditIssueType::TypeMismatch => {
                        if is_ja { "型ミスマッチ" }
                        else if is_zh_cn { "类型不匹配" }
                        else if is_zh_tw { "型態不匹配" }
                        else if is_ko { "타입 불일치" }
                        else if is_et { "Tüübi lahknevus" }
                        else if is_vi { "Sai kiểu dữ liệu" }
                        else if is_es { "Discrepancia de tipo" }
                        else if is_fr { "Incompatibilité de type" }
                        else if is_de { "Typkonflikt" }
                        else { "Type Mismatch" }
                    }
                    AuditIssueType::ParamCountMismatch => {
                        if is_ja { "引数個数ミスマッチ" }
                        else if is_zh_cn { "参数数量不匹配" }
                        else if is_zh_tw { "參數數量不匹配" }
                        else if is_ko { "매개변수 개수 불일치" }
                        else if is_et { "Parameetrite arvu lahknevus" }
                        else if is_vi { "Số lượng tham số không khớp" }
                        else if is_es { "Discrepancia de parámetros" }
                        else if is_fr { "Incompatibilité du nombre de paramètres" }
                        else if is_de { "Parameteranzahl-Konflikt" }
                        else { "Parameter Count Mismatch" }
                    }
                    AuditIssueType::ReturnTypeMismatch => {
                        if is_ja { "戻り値ミスマッチ" }
                        else if is_zh_cn { "返回值类型不匹配" }
                        else if is_zh_tw { "傳回值型態不匹配" }
                        else if is_ko { "반환 타입 불일치" }
                        else if is_et { "Tagastustüübi lahknevus" }
                        else if is_vi { "Kiểu trả về không khớp" }
                        else if is_es { "Discrepancia de tipo de retorno" }
                        else if is_fr { "Incompatibilité du type de retour" }
                        else if is_de { "Rückgabetyp-Konflikt" }
                        else { "Return Type Mismatch" }
                    }
                    AuditIssueType::LineNumberMissing => {
                        if is_ja { "行番号未記載" }
                        else if is_zh_cn { "行号未填写" }
                        else if is_zh_tw { "行號未填寫" }
                        else if is_ko { "라인 번호 누락" }
                        else if is_et { "Reanumber puudub" }
                        else if is_vi { "Chưa ghi số dòng" }
                        else if is_es { "Falta número de línea" }
                        else if is_fr { "Numéro de ligne manquant" }
                        else if is_de { "Zeilennummer fehlt" }
                        else { "Line Number Missing" }
                    }
                    AuditIssueType::LineNumberMismatch => {
                        if is_ja { "行番号ミスマッチ" }
                        else if is_zh_cn { "行号不匹配" }
                        else if is_zh_tw { "行號不匹配" }
                        else if is_ko { "라인 번호 불일치" }
                        else if is_et { "Reanumbri lahknevus" }
                        else if is_vi { "Số dòng không khớp" }
                        else if is_es { "Discrepancia de número de línea" }
                        else if is_fr { "Incompatibilité de dynamic número de línea" }
                        else if is_de { "Zeilennummern-Konflikt" }
                        else { "Line Number Mismatch" }
                    }
                    AuditIssueType::DependencyNotUsed => {
                        if is_ja { "依存先未使用" }
                        else if is_zh_cn { "依赖项未使用" }
                        else if is_zh_tw { "依賴項未使用" }
                        else if is_ko { "의존성 미사용" }
                        else if is_et { "Kasutamata sõltuvus" }
                        else if is_vi { "Phụ thuộc chưa dùng" }
                        else if is_es { "Dependencia no utilizada" }
                        else if is_fr { "Définition manquante dans le code" }
                        else if is_de { "Abhängigkeit nicht verwendet" }
                        else { "Dependency Not Used" }
                    }
                    AuditIssueType::DeadCode => {
                        if is_ja { "デッドコード" }
                        else if is_zh_cn { "无用代码" }
                        else if is_zh_tw { "無用程式碼" }
                        else if is_ko { "데드 코드" }
                        else if is_et { "Surnud kood" }
                        else if is_vi { "Mã chết" }
                        else if is_es { "Código muerto" }
                        else if is_fr { "Code mort" }
                        else if is_de { "Toter Code" }
                        else { "Dead Code" }
                    }
                };

                let spec_path_buf = s_uri.to_file_path().unwrap_or_else(|_| PathBuf::from(""));
                let spec_relative = spec_path_buf.strip_prefix(&root_path)
                    .unwrap_or(&spec_path_buf)
                    .to_string_lossy();

                report_content.push_str(&format!(
                    "- [ ] **{}** (シンボル: `{}`)\n  - **内容**: {}\n  - **仕様書箇所**: [{}](file:///{}) L{}\n",
                    issue_type_str,
                    escape_markdown(&issue.name),
                    escape_markdown(&issue.message),
                    spec_relative,
                    spec_path_buf.to_string_lossy().replace('\\', "/"),
                    issue.spec_line
                ));
            }

            let _ = tokio::fs::write(&report_path, report_content).await;
        }
    }
}

fn normalize_url(url: &Url) -> Url {
    let mut s = url.to_string();
    if s.starts_with("file:///") && s.len() >= 10 {
        let drive_char = s.as_bytes()[8] as char;
        if drive_char.is_ascii_uppercase() && s.as_bytes()[9] == b':' {
            let lower = drive_char.to_ascii_lowercase();
            s.replace_range(8..9, &lower.to_string());
        }
    }
    Url::parse(&s).unwrap_or_else(|_| url.clone())
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

async fn collect_project_used_symbols(dir: &Path) -> std::collections::HashSet<String> {
    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols_in_dir_recursive(dir, &mut used_set).await;
    used_set
}

async fn collect_used_symbols_in_dir_recursive(dir: &Path, used_set: &mut std::collections::HashSet<String>) {
    let mut read_dir = match tokio::fs::read_dir(dir).await {
        Ok(d) => d,
        Err(_) => return,
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "target" || dir_name == "node_modules" || dir_name == "docs" || dir_name == ".git" {
                continue;
            }
            Box::pin(collect_used_symbols_in_dir_recursive(&path, used_set)).await;
        } else if path.is_file() {
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extension == "rs" || extension == "ts" || extension == "js" || extension == "py" || extension == "go"
                || extension == "c" || extension == "h" || extension == "cpp" || extension == "hpp"
                || extension == "cc" || extension == "cxx" || extension == "cs" || extension == "rb"
                || extension == "swift" || extension == "kt" || extension == "kts" || extension == "java"
            {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if extension == "rs" {
                        let mut ts_parser = tree_sitter::Parser::new();
                        if ts_parser.set_language(tree_sitter_rust::language()).is_ok() {
                            if let Some(tree) = ts_parser.parse(&content, None) {
                                walk_node_for_identifiers(tree.root_node(), &content, used_set);
                            }
                        }
                    } else {
                        let re = regex::Regex::new(r"\b[a-zA-Z_]\w*\b").unwrap();
                        for cap in re.captures_iter(&content) {
                            used_set.insert(cap[0].to_string());
                        }
                    }
                }
            }
        }
    }
}

fn walk_node_for_identifiers(node: tree_sitter::Node, source: &str, used_set: &mut std::collections::HashSet<String>) {
    let kind = node.kind();
    if kind == "identifier" || kind == "type_identifier" || kind == "scoped_identifier" || kind == "field_identifier" {
        let mut is_definition = false;
        if let Some(parent) = node.parent() {
            if let Some(name_node) = parent.child_by_field_name("name") {
                if name_node.start_byte() == node.start_byte() && name_node.end_byte() == node.end_byte() {
                    is_definition = true;
                }
            }
        }

        if !is_definition {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                let t = text.trim().to_string();
                if !t.is_empty() {
                    if t.contains("::") {
                        for part in t.split("::") {
                            used_set.insert(part.trim().to_string());
                        }
                    } else if t.contains('.') {
                        for part in t.split('.') {
                            used_set.insert(part.trim().to_string());
                        }
                    }
                    used_set.insert(t);
                }
            }
        }
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_node_for_identifiers(cursor.node(), source, used_set);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn escape_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "\\<")
        .replace('>', "\\>")
}

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[Docs Auditor Server Panic] {}", info);
        std::process::exit(1);
    }));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend {
        client,
        state: Arc::new(Mutex::new(Hfsm::new())),
        root_path: Arc::new(Mutex::new(None)),
        locale: Arc::new(Mutex::new("en".to_string())),
        issues_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        project_used_symbols: Arc::new(Mutex::new(None)),
    });

    Server::new(stdin, stdout, messages).serve(service).await;
}
