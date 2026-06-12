use crate::parser::{SymbolInfo, SymbolKind};

#[derive(Debug, Clone, PartialEq)]
pub enum AuditIssueType {
    MissingInCode,
    TypeMismatch,
    ParamCountMismatch,
    ReturnTypeMismatch,
    LineNumberMissing,
    LineNumberMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditIssue {
    pub name: String,
    pub issue_type: AuditIssueType,
    pub message: String,
    pub spec_line: usize, // 仕様書内のエラー行
    pub code_line_range: Option<(usize, usize)>,
    pub expected_line_range: Option<(usize, usize)>,
}

pub fn audit_symbols(spec_symbols: &[SymbolInfo], code_symbols: &[SymbolInfo]) -> Vec<AuditIssue> {
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
                    message: format!("仕様書に記載されているシンボル '{}' がコード内に見つかりません。", spec.name),
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
                        message: format!(
                            "シンボル '{}' の種類が一致しません。仕様書: {:?}, コード: {:?}",
                            spec.name, spec.kind, code.kind
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
                                    message: format!(
                                        "関数 '{}' の引数の個数が一致しません。仕様書: {}個, コード: {}個",
                                        spec.name, spec_params.len(), code_params.len()
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
                                            message: format!(
                                                "関数 '{}' の引数 '{}' の型が一致しません。仕様書: {}, コード: {}",
                                                spec.name, s_name, s_type, c_type
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
                                    message: format!(
                                        "関数 '{}' の戻り値の型が一致しません。仕様書: {}, コード: {}",
                                        spec.name, spec_ret, code_ret
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
                                        message: format!(
                                            "変数 '{}' の型が一致しません。仕様書: {}, コード: {}",
                                            spec.name, spec_type, code_type
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
                            message: format!("仕様書にシンボル '{}' の行番号が記載されていません。", spec.name),
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
                                    message: format!(
                                        "仕様書に記載されている行番号範囲 (L{}-{}) が実際のコードの行範囲 (L{}-{}) と一致しません。",
                                        spec_range.0, spec_range.1, code_range.0, code_range.1
                                    ),
                                    spec_line,
                                    code_line_range: Some(code_range),
                                    expected_line_range: Some(spec_range),
                                });
                            }
                        }
                    }
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
            }
        ];

        let issues = audit_symbols(&spec, &code);
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
            }
        ];
        let code = vec![];

        let issues = audit_symbols(&spec, &code);
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
            }
        ];

        let issues = audit_symbols(&spec, &code);
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
            }
        ];

        let issues = audit_symbols(&spec, &code);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, AuditIssueType::LineNumberMismatch);
        assert_eq!(issues[0].spec_line, 4);
    }
}
