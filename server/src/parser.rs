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
    pub dependencies: Option<Vec<String>>,  // 仕様書側: 依存先リスト
    pub used_symbols: Option<Vec<String>>,  // コード側: 使用識別子リスト
}

pub fn parse_code(code: &str, lang: &str) -> Vec<SymbolInfo> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => parse_rust_code(code),
        "typescript" | "ts" | "tsx" | "javascript" | "js" | "jsx" => parse_typescript_code(code),
        "python" | "py" => parse_python_code(code),
        "go" => parse_go_code(code),
        "c" => parse_c_code(code),
        "cpp" => parse_cpp_code(code),
        "csharp" => parse_csharp_code(code),
        "ruby" => parse_ruby_code(code),
        "swift" => parse_swift_code(code),
        "kotlin" => parse_kotlin_code(code),
        "java" => parse_java_code(code),
        _ => Vec::new(),
    }
}

pub fn parse_typescript_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_typescript::language_typescript()).expect("Failed to load TypeScript language");
    
    let tree = parser.parse(code, None).expect("Failed to parse TypeScript code");
    let root_node = tree.root_node();
    
    let mut symbols = Vec::new();
    walk_typescript_node(root_node, code, &mut symbols);
    symbols
}

fn walk_typescript_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    
    if kind == "function_declaration" || kind == "method_definition" {
        if let Some(sym) = extract_ts_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "lexical_declaration" || kind == "variable_declaration" {
        extract_ts_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_typescript_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_ts_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "required_parameter" || child.kind() == "optional_parameter" || child.kind() == "formal_parameter" {
                    if let Some(pattern_node) = child.child_by_field_name("pattern") {
                        let pat_text = pattern_node.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string();
                        let type_text = child.child_by_field_name("type")
                            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                            .map(|t| t.trim().trim_start_matches(':').trim().to_string())
                            .unwrap_or_else(|| "any".to_string());
                        params.push((pat_text, type_text));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    
    let return_type = node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().trim_start_matches(':').trim().to_string());

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_ts_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        let name_str = name.trim().to_string();
                        let start_line = node.start_position().row + 1;
                        let end_line = node.end_position().row + 1;
                        
                        let var_type = child.child_by_field_name("type")
                            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                            .map(|t| t.trim().trim_start_matches(':').trim().to_string());

                        let mut used_set = std::collections::HashSet::new();
                        collect_used_symbols(child, source, &mut used_set);
                        used_set.remove(&name_str);
                        let used_symbols = Some(used_set.into_iter().collect());

                        symbols.push(SymbolInfo {
                            name: name_str,
                            kind: SymbolKind::Variable,
                            params: None,
                            return_type: None,
                            var_type,
                            line_range: Some((start_line, end_line)),
                            spec_line: None,
                            dependencies: None,
                            used_symbols,
                        });
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

pub fn parse_python_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_python::language()).expect("Failed to load Python language");
    
    let tree = parser.parse(code, None).expect("Failed to parse Python code");
    let root_node = tree.root_node();
    
    let mut symbols = Vec::new();
    walk_python_node(root_node, code, &mut symbols);
    symbols
}

fn walk_python_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    
    if kind == "function_definition" {
        if let Some(sym) = extract_python_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "assignment" {
        if let Some(sym) = extract_python_variable_info(node, source) {
            symbols.push(sym);
        }
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_python_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_python_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "typed_parameter" {
                    let mut name_opt = None;
                    let mut type_opt = None;
                    
                    let mut sub_cursor = child.walk();
                    if sub_cursor.goto_first_child() {
                        loop {
                            let sub_child = sub_cursor.node();
                            if sub_child.kind() == "identifier" {
                                if let Ok(n) = sub_child.utf8_text(source.as_bytes()) {
                                    name_opt = Some(n.trim().to_string());
                                }
                            }
                            if !sub_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    
                    if let Some(type_node) = child.child_by_field_name("type") {
                        if let Ok(t) = type_node.utf8_text(source.as_bytes()) {
                            type_opt = Some(t.trim().to_string());
                        }
                    }
                    
                    if let (Some(name_str), Some(type_str)) = (name_opt, type_opt) {
                        params.push((name_str, type_str));
                    }
                } else if child.kind() == "identifier" {
                    if let Ok(name_text) = child.utf8_text(source.as_bytes()) {
                        params.push((name_text.trim().to_string(), "any".to_string()));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    
    let return_type = node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().trim_start_matches("->").trim().to_string());

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_python_variable_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let left_node = node.child_by_field_name("left")?;
    let name: String;
    
    if left_node.kind() == "typed_parameter" {
        let name_node = left_node.child_by_field_name("name")?;
        name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    } else if left_node.kind() == "identifier" {
        name = left_node.utf8_text(source.as_bytes()).ok()?.to_string();
    } else {
        return None;
    }
    
    let var_type = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Variable,
        params: None,
        return_type: None,
        var_type,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

pub fn parse_go_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_go::language()).expect("Failed to load Go language");
    
    let tree = parser.parse(code, None).expect("Failed to parse Go code");
    let root_node = tree.root_node();
    
    let mut symbols = Vec::new();
    walk_go_node(root_node, code, &mut symbols);
    symbols
}

fn walk_go_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    
    if kind == "function_declaration" || kind == "method_declaration" {
        if let Some(sym) = extract_go_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "var_spec" || kind == "const_spec" {
        extract_go_spec_variable_info(node, source, symbols);
    } else if kind == "short_var_declaration" {
        extract_go_short_var_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_go_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_go_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parameter_declaration" {
                    let mut names = Vec::new();
                    let mut type_str = "any".to_string();
                    
                    let mut sub_cursor = child.walk();
                    if sub_cursor.goto_first_child() {
                        loop {
                            let sub_child = sub_cursor.node();
                            if sub_child.kind() == "identifier" {
                                if let Ok(n) = sub_child.utf8_text(source.as_bytes()) {
                                    names.push(n.trim().to_string());
                                }
                            } else {
                                if let Ok(t) = sub_child.utf8_text(source.as_bytes()) {
                                    type_str = t.trim().to_string();
                                }
                            }
                            if !sub_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    
                    for name in names {
                        params.push((name, type_str.clone()));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    
    let return_type = node.child_by_field_name("result")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_go_spec_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let mut names = Vec::new();
    let mut var_type = None;
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    names.push(name.trim().to_string());
                }
            } else if child.kind() != "literal" && child.kind() != "expression" {
                if let Some(t_node) = node.child_by_field_name("type") {
                    if let Ok(t) = t_node.utf8_text(source.as_bytes()) {
                        var_type = Some(t.trim().to_string());
                    }
                } else {
                    if child.kind().contains("type") {
                        if let Ok(t) = child.utf8_text(source.as_bytes()) {
                            var_type = Some(t.trim().to_string());
                        }
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    for name in names {
        let mut used_set = std::collections::HashSet::new();
        collect_used_symbols(node, source, &mut used_set);
        used_set.remove(&name);
        let used_symbols = Some(used_set.into_iter().collect());

        symbols.push(SymbolInfo {
            name,
            kind: SymbolKind::Variable,
            params: None,
            return_type: None,
            var_type: var_type.clone(),
            line_range: Some((start_line, end_line)),
            spec_line: None,
            dependencies: None,
            used_symbols,
        });
    }
}

fn extract_go_short_var_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let left_node = match node.child_by_field_name("left") {
        Some(n) => n,
        None => return,
    };
    
    let mut names = Vec::new();
    let mut cursor = left_node.walk();
    
    if left_node.kind() == "expression_list" {
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        names.push(name.trim().to_string());
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    } else if left_node.kind() == "identifier" {
        if let Ok(name) = left_node.utf8_text(source.as_bytes()) {
            names.push(name.trim().to_string());
        }
    }
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    for name in names {
        let mut used_set = std::collections::HashSet::new();
        collect_used_symbols(node, source, &mut used_set);
        used_set.remove(&name);
        let used_symbols = Some(used_set.into_iter().collect());

        symbols.push(SymbolInfo {
            name,
            kind: SymbolKind::Variable,
            params: None,
            return_type: None,
            var_type: None,
            line_range: Some((start_line, end_line)),
            spec_line: None,
            dependencies: None,
            used_symbols,
        });
    }
}

pub fn parse_markdown_spec(content: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();
    
    // 3段目以上の見出し（###や####など）で、`シンボル名` または `シンボル名` (Lxx-xx) にマッチする正規表現
    // バッククォートの前に他の文字（例: 関数:）があっても許容する
    let heading_regex = Regex::new(r"^#{3,}[^`]*`([^`]+)`(?:\s*\(L(\d+)(?:-(\d+))?\))?\s*$").unwrap();
    
    // 1. 変数や関数の見出しから宣言情報を抽出する
    for (idx, line) in content.lines().enumerate() {
        let physical_line = idx + 1;
        let trimmed = line.trim();
        
        if trimmed.starts_with("###") {
            if let Some(caps) = heading_regex.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                let mut line_range = None;
                
                if let Some(start_cap) = caps.get(2) {
                    let start = start_cap.as_str().parse::<usize>().unwrap();
                    let end = caps.get(3)
                        .map(|m| m.as_str().parse::<usize>().unwrap())
                        .unwrap_or(start);
                    line_range = Some((start, end));
                }
                
                symbols.push(SymbolInfo {
                    name,
                    kind: SymbolKind::Function, // デフォルトでFunction。型情報がないため kind ミスマッチは auditor 側でスキップする
                    params: None,
                    return_type: None,
                    var_type: None,
                    line_range,
                    spec_line: Some(physical_line),
                    dependencies: None,
                    used_symbols: None,
                });
            }
        }
    }

    // 2. Mermaid図から依存関係を抽出する
    let mut in_mermaid = false;
    let mermaid_regex = Regex::new(r"^\s*(\w+)(?:\[.*?\])?\s*-->\s*([\w:]+)(?:\[.*?\])?").unwrap();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```mermaid") {
            in_mermaid = true;
            continue;
        } else if in_mermaid && trimmed.starts_with("```") {
            in_mermaid = false;
            continue;
        }

        if in_mermaid {
            if let Some(caps) = mermaid_regex.captures(trimmed) {
                let caller = caps.get(1).unwrap().as_str().to_string();
                let callee = caps.get(2).unwrap().as_str().to_string();
                
                for sym in &mut symbols {
                    if sym.name == caller {
                        if sym.dependencies.is_none() {
                            sym.dependencies = Some(Vec::new());
                        }
                        if let Some(ref mut deps) = sym.dependencies {
                            if !deps.contains(&callee) {
                                deps.push(callee.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    symbols
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
    
    if kind == "function_item" {
        if let Some(sym) = extract_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "const_item" || kind == "static_item" {
        if let Some(sym) = extract_variable_info(node, source) {
            symbols.push(sym);
        }
    }

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

fn collect_used_symbols(node: Node, source: &str, used: &mut std::collections::HashSet<String>) {
    let kind = node.kind();
    if kind.ends_with("identifier") {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            let t = text.trim().to_string();
            if !t.is_empty() {
                used.insert(t);
            }
        }
    }
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_used_symbols(cursor.node(), source, used);
            if !cursor.goto_next_sibling() {
                break;
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

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
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

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Variable,
        params: None,
        return_type: None,
        var_type,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

pub fn parse_c_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_c::language()).expect("Failed to load C language");
    let tree = parser.parse(code, None).expect("Failed to parse C code");
    let mut symbols = Vec::new();
    walk_c_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_c_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "function_definition" {
        if let Some(sym) = extract_c_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "declaration" {
        extract_c_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_c_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_c_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let decl_node = node.child_by_field_name("declarator")?;
    
    let mut fn_declarator = decl_node;
    while fn_declarator.kind() == "pointer_declarator" {
        fn_declarator = fn_declarator.child_by_field_name("declarator")?;
    }
    
    let name_node = fn_declarator.child_by_field_name("declarator")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let return_type = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
        
    let mut params = Vec::new();
    if let Some(params_node) = fn_declarator.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parameter_declaration" {
                    let type_str = child.child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "any".to_string());
                    
                    let param_name = child.child_by_field_name("declarator")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "".to_string());
                        
                    if !param_name.is_empty() {
                        params.push((param_name, type_str));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_c_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let type_str = match node.child_by_field_name("type") {
        Some(t_node) => t_node.utf8_text(source.as_bytes()).ok().map(|t| t.trim().to_string()),
        None => return,
    };
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "init_declarator" {
                if let Some(decl_node) = child.child_by_field_name("declarator") {
                    let mut actual_decl = decl_node;
                    while actual_decl.kind() == "pointer_declarator" {
                        actual_decl = actual_decl.child_by_field_name("declarator").unwrap_or(actual_decl);
                    }
                    if let Ok(name) = actual_decl.utf8_text(source.as_bytes()) {
                        let name_str = name.trim().to_string();
                        let mut used_set = std::collections::HashSet::new();
                        collect_used_symbols(child, source, &mut used_set);
                        used_set.remove(&name_str);
                        
                        symbols.push(SymbolInfo {
                            name: name_str,
                            kind: SymbolKind::Variable,
                            params: None,
                            return_type: None,
                            var_type: type_str.clone(),
                            line_range: Some((start_line, end_line)),
                            spec_line: None,
                            dependencies: None,
                            used_symbols: Some(used_set.into_iter().collect()),
                        });
                    }
                }
            } else if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    let name_str = name.trim().to_string();
                    symbols.push(SymbolInfo {
                        name: name_str,
                        kind: SymbolKind::Variable,
                        params: None,
                        return_type: None,
                        var_type: type_str.clone(),
                        line_range: Some((start_line, end_line)),
                        spec_line: None,
                        dependencies: None,
                        used_symbols: Some(Vec::new()),
                    });
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

pub fn parse_cpp_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_cpp::language()).expect("Failed to load C++ language");
    let tree = parser.parse(code, None).expect("Failed to parse C++ code");
    let mut symbols = Vec::new();
    walk_cpp_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_cpp_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "function_definition" {
        if let Some(sym) = extract_cpp_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "declaration" {
        extract_cpp_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_cpp_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_cpp_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    extract_c_function_info(node, source)
}

fn extract_cpp_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    extract_c_variable_info(node, source, symbols);
}

pub fn parse_csharp_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_c_sharp::language()).expect("Failed to load C# language");
    let tree = parser.parse(code, None).expect("Failed to parse C# code");
    let mut symbols = Vec::new();
    walk_csharp_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_csharp_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "method_declaration" {
        if let Some(sym) = extract_csharp_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "variable_declaration" || kind == "field_declaration" {
        extract_csharp_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_csharp_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_csharp_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let return_type = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
        
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parameter" {
                    let type_str = child.child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "any".to_string());
                    
                    let param_name = child.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "".to_string());
                        
                    if !param_name.is_empty() {
                        params.push((param_name, type_str));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_csharp_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    // node が field_declaration の場合は、まず内部の variable_declaration を探す
    let target_node = if node.kind() == "field_declaration" {
        node.child_by_field_name("declaration").unwrap_or(node)
    } else {
        node
    };

    let type_str = target_node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
        
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let mut cursor = target_node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                let name_node = child.child_by_field_name("name")
                    .or_else(|| child.child(0)); // name フィールドが無い場合は最初の子 (通常 identifier)
                if let Some(n_node) = name_node {
                    if let Ok(name) = n_node.utf8_text(source.as_bytes()) {
                        let name_str = name.trim().to_string();
                        let mut used_set = std::collections::HashSet::new();
                        collect_used_symbols(child, source, &mut used_set);
                        used_set.remove(&name_str);
                        
                        symbols.push(SymbolInfo {
                            name: name_str,
                            kind: SymbolKind::Variable,
                            params: None,
                            return_type: None,
                            var_type: type_str.clone(),
                            line_range: Some((start_line, end_line)),
                            spec_line: None,
                            dependencies: None,
                            used_symbols: Some(used_set.into_iter().collect()),
                        });
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

pub fn parse_ruby_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_ruby::language()).expect("Failed to load Ruby language");
    let tree = parser.parse(code, None).expect("Failed to parse Ruby code");
    let mut symbols = Vec::new();
    walk_ruby_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_ruby_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "method" {
        if let Some(sym) = extract_ruby_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "assignment" {
        if let Some(sym) = extract_ruby_variable_info(node, source) {
            symbols.push(sym);
        }
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_ruby_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_ruby_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "identifier" {
                    if let Ok(p_name) = child.utf8_text(source.as_bytes()) {
                        params.push((p_name.trim().to_string(), "any".to_string()));
                    }
                } else if child.kind() == "formal_parameters" {
                    let mut sub_cursor = child.walk();
                    if sub_cursor.goto_first_child() {
                        loop {
                            let sub_child = sub_cursor.node();
                            if sub_child.kind() == "identifier" {
                                if let Ok(p_name) = sub_child.utf8_text(source.as_bytes()) {
                                    params.push((p_name.trim().to_string(), "any".to_string()));
                                }
                            }
                            if !sub_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type: Some("any".to_string()),
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_ruby_variable_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let left_node = node.child_by_field_name("left")?;
    let name = left_node.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    
    if left_node.kind() != "identifier" && left_node.kind() != "instance_variable" {
        return None;
    }
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Variable,
        params: None,
        return_type: None,
        var_type: Some("any".to_string()),
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols: Some(used_set.into_iter().collect()),
    })
}

pub fn parse_swift_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_swift::language()).expect("Failed to load Swift language");
    let tree = parser.parse(code, None).expect("Failed to parse Swift code");
    let mut symbols = Vec::new();
    walk_swift_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_swift_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "function_declaration" {
        if let Some(sym) = extract_swift_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "variable_declaration" || kind == "constant_declaration" || kind == "property_declaration" {
        extract_swift_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_swift_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_swift_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let mut name = String::new();
    let mut return_type = None;
    let mut params = Vec::new();
    let mut has_parameters = false;
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "simple_identifier" && name.is_empty() {
                name = child.utf8_text(source.as_bytes()).ok()?.trim().to_string();
            } else if child.kind() == "parameter" {
                has_parameters = true;
                let mut p_name = String::new();
                let mut p_type = "any".to_string();
                
                let mut sub_cursor = child.walk();
                if sub_cursor.goto_first_child() {
                    loop {
                        let sub_child = sub_cursor.node();
                        let field_name = sub_cursor.field_name();
                        if field_name == Some("name") && p_name.is_empty() {
                            p_name = sub_child.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string();
                        } else if sub_child.kind() == "user_type" || sub_child.kind().contains("type") {
                            p_type = sub_child.utf8_text(source.as_bytes()).unwrap_or("any").trim().to_string();
                        }
                        if !sub_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                if !p_name.is_empty() {
                    params.push((p_name, p_type));
                }
            } else if (child.kind() == "user_type" || child.kind().contains("type")) && has_parameters {
                return_type = Some(child.utf8_text(source.as_bytes()).ok()?.trim().to_string());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_swift_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut name_opt = None;
    let mut type_opt = None;
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "pattern" {
                let mut sub_cursor = child.walk();
                let mut found = false;
                if sub_cursor.goto_first_child() {
                    loop {
                        let sub_child = sub_cursor.node();
                        if sub_child.kind() == "bound_identifier" {
                            if let Some(simple_id) = sub_child.child(0) {
                                if simple_id.kind() == "simple_identifier" {
                                    name_opt = Some(simple_id.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string());
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !sub_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                if !found {
                    if let Ok(n) = child.utf8_text(source.as_bytes()) {
                        name_opt = Some(n.trim().to_string());
                    }
                }
            } else if child.kind() == "type_annotation" {
                let mut sub_cursor = child.walk();
                if sub_cursor.goto_first_child() {
                    loop {
                        let sub_child = sub_cursor.node();
                        if sub_child.kind() == "user_type" || sub_child.kind().contains("type") {
                            type_opt = Some(sub_child.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string());
                            break;
                        }
                        if !sub_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    if let Some(name) = name_opt {
        let mut used_set = std::collections::HashSet::new();
        collect_used_symbols(node, source, &mut used_set);
        used_set.remove(&name);
        
        symbols.push(SymbolInfo {
            name,
            kind: SymbolKind::Variable,
            params: None,
            return_type: None,
            var_type: type_opt,
            line_range: Some((start_line, end_line)),
            spec_line: None,
            dependencies: None,
            used_symbols: Some(used_set.into_iter().collect()),
        });
    }
}

pub fn parse_kotlin_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_kotlin::language()).expect("Failed to load Kotlin language");
    let tree = parser.parse(code, None).expect("Failed to parse Kotlin code");
    let mut symbols = Vec::new();
    walk_kotlin_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_kotlin_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "function_declaration" {
        if let Some(sym) = extract_kotlin_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "property_declaration" {
        extract_kotlin_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_kotlin_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_kotlin_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let mut name = String::new();
    let mut return_type = None;
    let mut params_node = None;
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "simple_identifier" && name.is_empty() {
                name = child.utf8_text(source.as_bytes()).ok()?.trim().to_string();
            } else if child.kind() == "function_value_parameters" {
                params_node = Some(child);
            } else if child.kind() == "user_type" || child.kind().contains("type") {
                if params_node.is_some() {
                    return_type = Some(child.utf8_text(source.as_bytes()).ok()?.trim().to_string());
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut params = Vec::new();
    if let Some(pn) = params_node {
        let mut cursor = pn.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "parameter" {
                    let mut p_name = String::new();
                    let mut p_type = "any".to_string();
                    
                    let mut sub_cursor = child.walk();
                    if sub_cursor.goto_first_child() {
                        loop {
                            let sub_child = sub_cursor.node();
                            if sub_child.kind() == "simple_identifier" {
                                p_name = sub_child.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string();
                            } else if sub_child.kind() == "user_type" || sub_child.kind().contains("type") {
                                p_type = sub_child.utf8_text(source.as_bytes()).unwrap_or("any").trim().to_string();
                            }
                            if !sub_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    if !p_name.is_empty() {
                        params.push((p_name, p_type));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_kotlin_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let mut name_opt = None;
    let mut type_opt = None;
    
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declaration" {
                let mut sub_cursor = child.walk();
                if sub_cursor.goto_first_child() {
                    loop {
                        let sub_child = sub_cursor.node();
                        if sub_child.kind() == "simple_identifier" {
                            name_opt = Some(sub_child.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string());
                        } else if sub_child.kind() == "user_type" || sub_child.kind().contains("type") {
                            type_opt = Some(sub_child.utf8_text(source.as_bytes()).unwrap_or("").trim().to_string());
                        }
                        if !sub_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    if let Some(name) = name_opt {
        let mut used_set = std::collections::HashSet::new();
        collect_used_symbols(node, source, &mut used_set);
        used_set.remove(&name);
        
        symbols.push(SymbolInfo {
            name,
            kind: SymbolKind::Variable,
            params: None,
            return_type: None,
            var_type: type_opt,
            line_range: Some((start_line, end_line)),
            spec_line: None,
            dependencies: None,
            used_symbols: Some(used_set.into_iter().collect()),
        });
    }
}

pub fn parse_java_code(code: &str) -> Vec<SymbolInfo> {
    let mut parser = TsParser::new();
    parser.set_language(tree_sitter_java::language()).expect("Failed to load Java language");
    let tree = parser.parse(code, None).expect("Failed to parse Java code");
    let mut symbols = Vec::new();
    walk_java_node(tree.root_node(), code, &mut symbols);
    symbols
}

fn walk_java_node(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let kind = node.kind();
    if kind == "method_declaration" {
        if let Some(sym) = extract_java_function_info(node, source) {
            symbols.push(sym);
        }
    } else if kind == "field_declaration" || kind == "local_variable_declaration" {
        extract_java_variable_info(node, source, symbols);
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_java_node(cursor.node(), source, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_java_function_info(node: Node, source: &str) -> Option<SymbolInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(source.as_bytes()).ok()?.trim().to_string();
    
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let return_type = node.child_by_field_name("dimensions")
        .or_else(|| node.child_by_field_name("type"))
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
        
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "formal_parameter" {
                    let type_str = child.child_by_field_name("type")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "any".to_string());
                    
                    let param_name = child.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| "".to_string());
                        
                    if !param_name.is_empty() {
                        params.push((param_name, type_str));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let mut used_set = std::collections::HashSet::new();
    collect_used_symbols(node, source, &mut used_set);
    used_set.remove(&name);
    let used_symbols = Some(used_set.into_iter().collect());

    Some(SymbolInfo {
        name,
        kind: SymbolKind::Function,
        params: Some(params),
        return_type,
        var_type: None,
        line_range: Some((start_line, end_line)),
        spec_line: None,
        dependencies: None,
        used_symbols,
    })
}

fn extract_java_variable_info(node: Node, source: &str, symbols: &mut Vec<SymbolInfo>) {
    let type_str = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|t| t.trim().to_string());
        
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        let name_str = name.trim().to_string();
                        let mut used_set = std::collections::HashSet::new();
                        collect_used_symbols(child, source, &mut used_set);
                        used_set.remove(&name_str);
                        
                        symbols.push(SymbolInfo {
                            name: name_str,
                            kind: SymbolKind::Variable,
                            params: None,
                            return_type: None,
                            var_type: type_str.clone(),
                            line_range: Some((start_line, end_line)),
                            spec_line: None,
                            dependencies: None,
                            used_symbols: Some(used_set.into_iter().collect()),
                        });
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_spec() {
        let md = r#"# Test Spec
## 関数定義

### `hello`
- **引数**:
  - `user: String`

### `timeout` (L30)

### `FileLocker` (構造体)

```mermaid
graph TD
    hello --> parser::parse_markdown_spec
    hello --> Hfsm
```
"#;
        let symbols = parse_markdown_spec(md);
        // `FileLocker` はカテゴリ (構造体) があるので除外され、hello と timeout の2つだけが抽出される
        assert_eq!(symbols.len(), 2);

        // 1つ目のシンボル
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].line_range, None);
        assert_eq!(symbols[0].spec_line, Some(4)); // 4行目 (### `hello`)
        assert_eq!(symbols[0].dependencies, Some(vec!["parser::parse_markdown_spec".to_string(), "Hfsm".to_string()]));

        // 2つ目のシンボル
        assert_eq!(symbols[1].name, "timeout");
        assert_eq!(symbols[1].line_range, Some((30, 30)));
        assert_eq!(symbols[1].spec_line, Some(8)); // 8行目 (### `timeout` (L30))
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

    #[test]
    fn test_parse_typescript_code() {
        let code = r#"
const DEFAULT_LIMIT = 50;
function processUser(id: string, age: number): boolean {
    return true;
}
        "#;
        let symbols = parse_typescript_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "processUser");
        assert_eq!(func.params, Some(vec![("id".to_string(), "string".to_string()), ("age".to_string(), "number".to_string())]));
        assert_eq!(func.return_type, Some("boolean".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "DEFAULT_LIMIT");
    }

    #[test]
    fn test_parse_python_code() {
        let code = r#"
TIMEOUT: int = 30

def greet_user(name: str) -> str:
    return "Hello " + name
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_python::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("PYTHON AST: {}", tree.root_node().to_sexp());

        let symbols = parse_python_code(code);
        println!("PYTHON SYMBOLS: {:?}", symbols);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "greet_user");
        assert_eq!(func.params, Some(vec![("name".to_string(), "str".to_string())]));
        assert_eq!(func.return_type, Some("str".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "TIMEOUT");
        assert_eq!(var.var_type, Some("int".to_string()));
    }

    #[test]
    fn test_parse_go_code() {
        let code = r#"
package main

const Version string = "1.0.0"

func AddValues(x, y int) int {
    z := x + y
    return z
}
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_go::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("GO AST: {}", tree.root_node().to_sexp());

        let symbols = parse_go_code(code);
        println!("GO SYMBOLS: {:?}", symbols);
        assert!(symbols.len() >= 3);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "AddValues");
        assert_eq!(func.params, Some(vec![("x".to_string(), "int".to_string()), ("y".to_string(), "int".to_string())]));
        assert_eq!(func.return_type, Some("int".to_string()));
        
        let var = symbols.iter().find(|s| s.name == "Version").unwrap();
        assert_eq!(var.var_type, Some("string".to_string()));
        
        let short_var = symbols.iter().find(|s| s.name == "z").unwrap();
        assert_eq!(short_var.kind, SymbolKind::Variable);
    }

    #[test]
    fn test_parse_c_code() {
        let code = r#"
int max_limit = 200;
int calculate_area(int width, int height) {
    return width * height;
}
        "#;
        let symbols = parse_c_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "calculate_area");
        assert_eq!(func.params, Some(vec![("width".to_string(), "int".to_string()), ("height".to_string(), "int".to_string())]));
        assert_eq!(func.return_type, Some("int".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "max_limit");
        assert_eq!(var.var_type, Some("int".to_string()));
    }

    #[test]
    fn test_parse_cpp_code() {
        let code = r#"
double pi = 3.14159;
double compute_circle(double radius) {
    return pi * radius * radius;
}
        "#;
        let symbols = parse_cpp_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "compute_circle");
        assert_eq!(func.params, Some(vec![("radius".to_string(), "double".to_string())]));
        assert_eq!(func.return_type, Some("double".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "pi");
        assert_eq!(var.var_type, Some("double".to_string()));
    }

    #[test]
    fn test_parse_csharp_code() {
        let code = r#"
class Demo {
    private static int DefaultScore = 100;
    public int Process(string input, int factor) {
        return factor * 2;
    }
}
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_c_sharp::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("CSHARP AST: {}", tree.root_node().to_sexp());

        let symbols = parse_csharp_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "Process");
        assert_eq!(func.params, Some(vec![("input".to_string(), "string".to_string()), ("factor".to_string(), "int".to_string())]));
        assert_eq!(func.return_type, Some("int".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "DefaultScore");
        assert_eq!(var.var_type, Some("int".to_string()));
    }

    #[test]
    fn test_parse_ruby_code() {
        let code = r#"
api_key = "secret"
def perform_request(url, timeout)
    puts url
end
        "#;
        let symbols = parse_ruby_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "perform_request");
        assert_eq!(func.params, Some(vec![("url".to_string(), "any".to_string()), ("timeout".to_string(), "any".to_string())]));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "api_key");
    }

    #[test]
    fn test_parse_swift_code() {
        let code = r#"
let greeting: String = "Hello"
func performAction(action: String, retries: Int) -> Bool {
    return true;
}
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_swift::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("SWIFT AST: {}", tree.root_node().to_sexp());

        let symbols = parse_swift_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "performAction");
        assert_eq!(func.params, Some(vec![("action".to_string(), "String".to_string()), ("retries".to_string(), "Int".to_string())]));
        assert_eq!(func.return_type, Some("Bool".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "greeting");
        assert_eq!(var.var_type, Some("String".to_string()));
    }

    #[test]
    fn test_parse_kotlin_code() {
        let code = r#"
val appName: String = "Auditor"
fun updateStatus(status: String, code: Int): Boolean {
    return false;
}
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_kotlin::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("KOTLIN AST: {}", tree.root_node().to_sexp());

        let symbols = parse_kotlin_code(code);
        assert_eq!(symbols.len(), 2);
        
        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "updateStatus");
        assert_eq!(func.params, Some(vec![("status".to_string(), "String".to_string()), ("code".to_string(), "Int".to_string())]));
        assert_eq!(func.return_type, Some("Boolean".to_string()));
        
        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "appName");
        assert_eq!(var.var_type, Some("String".to_string()));
    }

    #[test]
    fn test_parse_java_code() {
        let code = r#"
class Main {
    public static int baseScore = 90;
    public boolean processRequest(String user, int score) {
        return score >= baseScore;
    }
}
        "#;
        let mut parser = TsParser::new();
        parser.set_language(tree_sitter_java::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        println!("JAVA AST: {}", tree.root_node().to_sexp());

        let symbols = parse_java_code(code);
        assert_eq!(symbols.len(), 2);

        let func = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(func.name, "processRequest");
        assert_eq!(func.params, Some(vec![("user".to_string(), "String".to_string()), ("score".to_string(), "int".to_string())]));
        assert_eq!(func.return_type, Some("boolean".to_string()));

        let var = symbols.iter().find(|s| s.kind == SymbolKind::Variable).unwrap();
        assert_eq!(var.name, "baseScore");
        assert_eq!(var.var_type, Some("int".to_string()));
    }

    #[test]
    fn test_parse_markdown_spec_new() {
        let doc = r#"
# 仕様書タイトル

## セクション

### `my_func` (L10-20)
- 何か説明

### 関数:`my_func_with_prefix` (L25-30)
- 何か説明

#### `my_var_h4` (L35)
- 見出し4

##### 変数: `my_var_h5`
- 見出し5
        "#;
        let symbols = parse_markdown_spec(doc);
        assert_eq!(symbols.len(), 4);

        assert_eq!(symbols[0].name, "my_func");
        assert_eq!(symbols[0].line_range, Some((10, 20)));
        assert_eq!(symbols[0].spec_line, Some(6));

        assert_eq!(symbols[1].name, "my_func_with_prefix");
        assert_eq!(symbols[1].line_range, Some((25, 30)));
        assert_eq!(symbols[1].spec_line, Some(9));

        assert_eq!(symbols[2].name, "my_var_h4");
        assert_eq!(symbols[2].line_range, Some((35, 35)));
        assert_eq!(symbols[2].spec_line, Some(12));

        assert_eq!(symbols[3].name, "my_var_h5");
        assert_eq!(symbols[3].line_range, None);
        assert_eq!(symbols[3].spec_line, Some(15));
    }
}
