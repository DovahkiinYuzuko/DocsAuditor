use regex::Regex;
use tree_sitter::{Node, Parser as TsParser};

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub params: Option<Vec<(String, String)>>, // (name, type)
    pub return_type: Option<String>,
    pub var_type: Option<String>,
    pub line_range: Option<(usize, usize)>, // (start_line, end_line) - 1-indexed
    pub spec_line: Option<usize>,           // 仕様書内での定義物理行番号（1-indexed）
}

pub fn parse_markdown_spec(content: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();
    
    // 行ベースで解析して仕様書の物理行番号を正確に特定する
    for (idx, line) in content.lines().enumerate() {
        let physical_line = idx + 1;
        let trimmed = line.trim();
        
        // 箇条書き（- または *）で始まる行を解析対象とする
        if trimmed.starts_with('-') || trimmed.starts_with('*') {
            let item_content = trimmed[1..].trim();
            if let Some(mut sym) = parse_spec_line(item_content) {
                sym.spec_line = Some(physical_line);
                symbols.push(sym);
            }
        }
    }
    symbols
}

fn parse_spec_line(line: &str) -> Option<SymbolInfo> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 行番号記述 (L10-20) または (L10) を抽出
    let line_regex = Regex::new(r"\(L(\d+)(?:-(\d+))?\)\s*$").unwrap();
    let mut line_range = None;
    let mut clean_line = trimmed.to_string();

    if let Some(caps) = line_regex.captures(trimmed) {
        let start = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
        let end = caps.get(2)
            .map(|m| m.as_str().parse::<usize>().unwrap())
            .unwrap_or(start);
        line_range = Some((start, end));
        clean_line = line_regex.replace(trimmed, "").trim().to_string();
    }

    // 関数のパース: fn name(param: type, ...) -> ret_type
    let fn_regex = Regex::new(r"^fn\s+(\w+)\s*(?:\((.*?)\))?\s*(?:->\s*([^\s{]+))?").unwrap();
    if let Some(caps) = fn_regex.captures(&clean_line) {
        let name = caps.get(1).unwrap().as_str().to_string();
        
        let params = caps.get(2).map(|m| {
            let p_str = m.as_str();
            if p_str.trim().is_empty() {
                Vec::new()
            } else {
                p_str.split(',')
                    .map(|p| {
                        let parts: Vec<&str> = p.split(':').collect();
                        let p_name = parts.get(0).unwrap_or(&"").trim().to_string();
                        let p_type = parts.get(1).unwrap_or(&"").trim().to_string();
                        (p_name, p_type)
                    })
                    .collect()
            }
        });

        let return_type = caps.get(3).map(|m| m.as_str().to_string());

        return Some(SymbolInfo {
            name,
            kind: SymbolKind::Function,
            params,
            return_type,
            var_type: None,
            line_range,
            spec_line: None,
        });
    }

    // 変数・定数のパース: let name: type または const name: type
    let var_regex = Regex::new(r"^(?:let|const|static)\s+(\w+)\s*(?::\s*([^\s=]+))?").unwrap();
    if let Some(caps) = var_regex.captures(&clean_line) {
        let name = caps.get(1).unwrap().as_str().to_string();
        let var_type = caps.get(2).map(|m| m.as_str().to_string());

        return Some(SymbolInfo {
            name,
            kind: SymbolKind::Variable,
            params: None,
            return_type: None,
            var_type,
            line_range,
            spec_line: None,
        });
    }

    None
}

pub fn parse_rust_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_rust::language()).expect("Failed to load Rust language");
    
    let tree = parser.parse(code, None).expect("Failed to parse Rust code");
    let root_node = tree.root_node();
    
    let mut symbols = Vec::new();
    walk_rust_node(root_node, code, &mut symbols);
    symbols
}

fn walk_rust_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    
    match kind {
        "function_item" => {
            if let Some(sym) = extract_function_info(node, source) {
                symbols.push(sym);
            }
        }
        "const_item" | "static_item" => {
            if let Some(sym) = extract_variable_info(node, source) {
                symbols.push(sym);
            }
        }
        _ => {
            // 子ノードを再帰的に走査
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    walk_rust_node(cursor.node(), source, symbols);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
    }
}

fn extract_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    // 引数の抽出
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parameter" {
                    if let (Some(pat_node), Some(type_node)) = (child.child_by_field_name("pattern"), child.child_by_field_name("type")) {
                        if let (Ok(pat_text), Ok(type_text)) = (pat_node.utf8_text(source.as_bytes()), type_node.utf8_text(source.as_bytes())) {
                            params.push((pat_text.trim().to_string(), type_text.trim().to_string()));
                        }
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    // 戻り値の型
    let return_type = node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().trim_start_matches("->").trim().to_string());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
    })
}

fn extract_variable_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let var_type = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Variable,
        params: None,
        return_type: None,
        var_type,
        line_range: Some((start_line, end_line)),
        spec_line: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_spec() {
        let md = r#"# Test Spec
- fn hello(user: String) -> Result (L10-20)
- let timeout: u64 (L30)
- invalid item without fn or let
"#;
        let symbols = parse_markdown_spec(md);
        assert_eq!(symbols.len(), 2);

        // 1つ目のシンボル (Function)
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].params, Some(vec![("user".to_string(), "String".to_string())]));
        assert_eq!(symbols[0].return_type, Some("Result".to_string()));
        assert_eq!(symbols[0].line_range, Some((10, 20)));
        assert_eq!(symbols[0].spec_line, Some(2)); // 2行目

        // 2つ目のシンボル (Variable)
        assert_eq!(symbols[1].name, "timeout");
        assert_eq!(symbols[1].kind, SymbolKind::Variable);
        assert_eq!(symbols[1].var_type, Some("u64".to_string()));
        assert_eq!(symbols[1].line_range, Some((30, 30)));
        assert_eq!(symbols[1].spec_line, Some(3)); // 3行目
    }

    #[test]
    fn test_parse_rust_code() {
        let code = r#"
const DEFAULT_TIMEOUT: u32 = 100;

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}
        "#;
        let symbols = parse_rust_code(code);
        // calculate_sum と DEFAULT_TIMEOUT の2つ
        assert_eq!(symbols.len(), 2);

        let func_sym = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func_sym.name, "calculate_sum");
        assert_eq!(func_sym.params, Some(vec![("a".to_string(), "i32".to_string()), ("b".to_string(), "i32".to_string())]));
        assert_eq!(func_sym.return_type, Some("i32".to_string()));

        let var_sym = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var_sym.name, "DEFAULT_TIMEOUT");
        assert_eq!(var_sym.var_type, Some("u32".to_string()));
    }
}
