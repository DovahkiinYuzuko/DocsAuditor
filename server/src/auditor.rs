use crate::parser::{SymbolInfo, SymbolKind};
use crate::i18n::{get_message, MessageKey};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AuditIssueType {
    MissingInCode,
    TypeMismatch,
    ParamCountMismatch,
    ReturnTypeMismatch,
    LineNumberMissing,
    LineNumberMismatch,
    DependencyNotUsed,
    DeadCode,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuditIssue {
    pub name: String,
    pub issue_type: AuditIssueType,
    pub message: String,
    pub spec_line: usize, // 仕様書内のエラー行
    pub code_line_range: Option<(usize, usize)>,
    pub expected_line_range: Option<(usize, usize)>,
}

pub fn audit_symbols(spec_symbols: &[SymbolInfo], code_symbols: &[SymbolInfo], project_used_symbols: &std::collections::HashSet<String>, locale: &str) -> Vec<AuditIssue> {
    let mut issues = Vec::new();

    for spec in spec_symbols {
        let spec_line = spec.spec_line.unwrap_or(1);

        // 同じ名前のシンボルをコード側から探す
        let code_match = code_symbols.iter().find(|c| c.name == spec.name);

        match code_match {
            None => {
                // コード側に対応する定義が存在しない
                issues.push(AuditIssue {
                    name: spec.name.clone(),
                    issue_type: AuditIssueType::MissingInCode,
                    message: get_message(&MessageKey::MissingInCode(spec.name.clone()), locale),
                    spec_line,
                    code_line_range: None,
                    expected_line_range: spec.line_range,
                });
            }
            Some(code) => {
                // 1. 変数/関数の種類のチェック
                if spec.kind != code.kind {
                    issues.push(AuditIssue {
                        name: spec.name.clone(),
                        issue_type: AuditIssueType::TypeMismatch,
                        message: get_message(
                            &MessageKey::KindMismatch(
                                spec.name.clone(),
                                format!("{:?}", spec.kind),
                                format!("{:?}", code.kind),
                            ),
                            locale,
                        ),
                        spec_line,
                        code_line_range: code.line_range,
                        expected_line_range: spec.line_range,
                    });
                    continue;
                }

                // 2. 引数のチェック (関数の場合)
                if spec.kind == SymbolKind::Function {
                    // 引数の個数と型のチェック (仕様書に引数が明記されている場合のみ)
                    if let Some(spec_params) = &spec.params {
                        if let Some(code_params) = &code.params {
                            if spec_params.len() != code_params.len() {
                                issues.push(AuditIssue {
                                    name: spec.name.clone(),
                                    issue_type: AuditIssueType::ParamCountMismatch,
                                    message: get_message(
                                        &MessageKey::ParamCountMismatch(
                                            spec.name.clone(),
                                            spec_params.len(),
                                            code_params.len(),
                                        ),
                                        locale,
                                    ),
                                    spec_line,
                                    code_line_range: code.line_range,
                                    expected_line_range: spec.line_range,
                                });
                            } else {
                                // 各引数の型チェック (型名が一致するか、空でない場合のみ)
                                for (i, (s_name, s_type)) in spec_params.iter().enumerate() {
                                    let (_, c_type) = &code_params[i];
                                    if !s_type.is_empty() && s_type != c_type {
                                        issues.push(AuditIssue {
                                            name: spec.name.clone(),
                                            issue_type: AuditIssueType::TypeMismatch,
                                            message: get_message(
                                                &MessageKey::TypeMismatch(
                                                    spec.name.clone(),
                                                    s_name.clone(),
                                                    s_type.clone(),
                                                    c_type.clone(),
                                                ),
                                                locale,
                                            ),
                                            spec_line,
                                            code_line_range: code.line_range,
                                            expected_line_range: spec.line_range,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // 戻り値のチェック (仕様書に戻り値が明記されている場合のみ)
                    if let Some(spec_ret) = &spec.return_type {
                        if !spec_ret.is_empty() {
                            let code_ret = code.return_type.as_deref().unwrap_or("()");
                            if spec_ret != code_ret {
                                issues.push(AuditIssue {
                                    name: spec.name.clone(),
                                    issue_type: AuditIssueType::ReturnTypeMismatch,
                                    message: get_message(
                                        &MessageKey::ReturnTypeMismatch(
                                            spec.name.clone(),
                                            spec_ret.clone(),
                                            code_ret.to_string(),
                                        ),
                                        locale,
                                    ),
                                    spec_line,
                                    code_line_range: code.line_range,
                                    expected_line_range: spec.line_range,
                                });
                            }
                        }
                    }
                } else {
                    // 変数の型チェック (仕様書に型が明記されている場合のみ)
                    if let Some(spec_type) = &spec.var_type {
                        if !spec_type.is_empty() {
                            if let Some(code_type) = &code.var_type {
                                if spec_type != code_type {
                                    issues.push(AuditIssue {
                                        name: spec.name.clone(),
                                        issue_type: AuditIssueType::TypeMismatch,
                                        message: get_message(
                                            &MessageKey::VarTypeMismatch(
                                                spec.name.clone(),
                                                spec_type.clone(),
                                                code_type.clone(),
                                            ),
                                            locale,
                                        ),
                                        spec_line,
                                        code_line_range: code.line_range,
                                        expected_line_range: spec.line_range,
                                    });
                                }
                            }
                        }
                    }
                }

                // 3. 行番号のチェック
                match spec.line_range {
                    None => {
                        // 行番号未記載
                        issues.push(AuditIssue {
                            name: spec.name.clone(),
                            issue_type: AuditIssueType::LineNumberMissing,
                            message: get_message(
                                &MessageKey::LineNumberMissing(spec.name.clone()),
                                locale,
                            ),
                            spec_line,
                            code_line_range: code.line_range,
                            expected_line_range: None,
                        });
                    }
                    Some(spec_range) => {
                        if let Some(code_range) = code.line_range {
                            if spec_range != code_range {
                                // 行番号ミスマッチ
                                issues.push(AuditIssue {
                                    name: spec.name.clone(),
                                    issue_type: AuditIssueType::LineNumberMismatch,
                                    message: get_message(
                                        &MessageKey::LineNumberMismatch(
                                            spec.name.clone(),
                                            format!("L{}-{}", spec_range.0, spec_range.1),
                                            format!("L{}-{}", code_range.0, code_range.1),
                                        ),
                                        locale,
                                    ),
                                    spec_line,
                                    code_line_range: Some(code_range),
                                    expected_line_range: Some(spec_range),
                                });
                            }
                        }
                    }
                }

                // 4. 依存関係のチェック
                if let Some(ref deps) = spec.dependencies {
                    for dep in deps {
                        let has_dep = if let Some(ref used) = code.used_symbols {
                            used.contains(dep) || {
                                if let Some(last_part) = dep.split("::").last() {
                                    used.contains(&last_part.to_string())
                                } else {
                                    false
                                }
                            }
                        } else {
                            false
                        };

                        if !has_dep {
                            issues.push(AuditIssue {
                                name: spec.name.clone(),
                                issue_type: AuditIssueType::DependencyNotUsed,
                                message: get_message(
                                    &MessageKey::DependencyNotUsed(spec.name.clone(), dep.clone()),
                                    locale,
                                ),
                                spec_line,
                                code_line_range: code.line_range,
                                expected_line_range: None,
                            });
                        }
                    }
                }

                // 5. デッドコードのチェック
                if !project_used_symbols.contains(&spec.name) {
                    issues.push(AuditIssue {
                        name: spec.name.clone(),
                        issue_type: AuditIssueType::DeadCode,
                        message: get_message(
                            &MessageKey::DeadCode(spec.name.clone()),
                            locale,
                        ),
                        spec_line,
                        code_line_range: code.line_range,
                        expected_line_range: None,
                    });
                }
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SymbolKind;
    use crate::parser::SymbolInfo;
    use std::collections::HashSet;

    fn make_used_set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_audit_symbols_success() {
        let spec = vec![
            SymbolInfo {
                name: "login".to_string(),
                kind: SymbolKind::Function,
                params: Some(vec![("user".to_string(), "String".to_string())]),
                return_type: Some("Result".to_string()),
                var_type: None,
                line_range: Some((10, 20)),
                spec_line: Some(2),
                dependencies: None,
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "login".to_string(),
                kind: SymbolKind::Function,
                params: Some(vec![("user".to_string(), "String".to_string())]),
                return_type: Some("Result".to_string()),
                var_type: None,
                line_range: Some((10, 20)),
                spec_line: None,
                dependencies: None,
                used_symbols: None,
            }
        ];

        let project_used = make_used_set(&["login"]);
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_audit_symbols_missing_in_code() {
        let spec = vec![
            SymbolInfo {
                name: "logout".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: None,
                spec_line: Some(5),
                dependencies: None,
                used_symbols: None,
            }
        ];
        let code = vec![];

        let project_used = HashSet::new();
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::MissingInCode);
        assert_eq!(issues[0].spec_line, 5);
    }

    #[test]
    fn test_audit_symbols_type_mismatch() {
        let spec = vec![
            SymbolInfo {
                name: "timeout".to_string(),
                kind: SymbolKind::Variable,
                params: None,
                return_type: None,
                var_type: Some("u64".to_string()),
                line_range: Some((30, 30)),
                spec_line: Some(10),
                dependencies: None,
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "timeout".to_string(),
                kind: SymbolKind::Variable,
                params: None,
                return_type: None,
                var_type: Some("u32".to_string()),
                line_range: Some((30, 30)),
                spec_line: None,
                dependencies: None,
                used_symbols: None,
            }
        ];

        let project_used = make_used_set(&["timeout"]);
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::TypeMismatch);
        assert_eq!(issues[0].spec_line, 10);
    }

    #[test]
    fn test_audit_symbols_line_mismatch() {
        let spec = vec![
            SymbolInfo {
                name: "login".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((10, 20)),
                spec_line: Some(4),
                dependencies: None,
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "login".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((15, 25)),
                spec_line: None,
                dependencies: None,
                used_symbols: None,
            }
        ];

        let project_used = make_used_set(&["login"]);
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::LineNumberMismatch);
        assert_eq!(issues[0].spec_line, 4);
    }

    #[test]
    fn test_audit_symbols_dependency_success() {
        let spec = vec![
            SymbolInfo {
                name: "on_change".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: Some(5),
                dependencies: Some(vec!["parser::parse_markdown_spec".to_string()]),
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "on_change".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: None,
                dependencies: None,
                used_symbols: Some(vec!["parse_markdown_spec".to_string()]),
            }
        ];

        let project_used = make_used_set(&["on_change"]);
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert!(issues.is_empty());
    }

    #[test]
    fn test_audit_symbols_dependency_failed() {
        let spec = vec![
            SymbolInfo {
                name: "on_change".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: Some(5),
                dependencies: Some(vec!["parser::parse_markdown_spec".to_string()]),
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "on_change".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: None,
                dependencies: None,
                used_symbols: Some(vec!["another_function".to_string()]),
            }
        ];

        let project_used = make_used_set(&["on_change"]);
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::DependencyNotUsed);
        assert_eq!(issues[0].spec_line, 5);
    }

    #[test]
    fn test_audit_symbols_dead_code() {
        let spec = vec![
            SymbolInfo {
                name: "unused_func".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: Some(3),
                dependencies: None,
                used_symbols: None,
            }
        ];
        let code = vec![
            SymbolInfo {
                name: "unused_func".to_string(),
                kind: SymbolKind::Function,
                params: None,
                return_type: None,
                var_type: None,
                line_range: Some((1, 10)),
                spec_line: None,
                dependencies: None,
                used_symbols: None,
            }
        ];

        let project_used = HashSet::new();
        let issues = audit_symbols(&spec, &code, &project_used, "ja");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::DeadCode);
        assert_eq!(issues[0].spec_line, 3);
    }
}
