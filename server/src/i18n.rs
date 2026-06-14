use std::collections::HashMap;
use std::sync::OnceLock;

pub enum MessageKey {
    MissingInCode(String),
    KindMismatch(String, String, String),
    TypeMismatch(String, String, String, String),
    VarTypeMismatch(String, String, String),
    ParamCountMismatch(String, usize, usize),
    ReturnTypeMismatch(String, String, String),
    LineNumberMissing(String),
    LineNumberMismatch(String, String, String),
    DependencyNotUsed(String, String),
    DeadCode(String),
    ReportTitle,
    ReportHeader,
    ReportSectionTitle,
    CodeActionTitle(String),
}

static LOCALES: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();

fn load_locales() -> HashMap<String, HashMap<String, String>> {
    let mut m = HashMap::new();
    m.insert("en".to_string(), serde_json::from_str(include_str!("../locales/en.json")).unwrap());
    m.insert("ja".to_string(), serde_json::from_str(include_str!("../locales/ja.json")).unwrap());
    m.insert("zh-cn".to_string(), serde_json::from_str(include_str!("../locales/zh-cn.json")).unwrap());
    m.insert("zh-tw".to_string(), serde_json::from_str(include_str!("../locales/zh-tw.json")).unwrap());
    m.insert("ko".to_string(), serde_json::from_str(include_str!("../locales/ko.json")).unwrap());
    m.insert("et".to_string(), serde_json::from_str(include_str!("../locales/et.json")).unwrap());
    m.insert("vi".to_string(), serde_json::from_str(include_str!("../locales/vi.json")).unwrap());
    m.insert("es".to_string(), serde_json::from_str(include_str!("../locales/es.json")).unwrap());
    m.insert("fr".to_string(), serde_json::from_str(include_str!("../locales/fr.json")).unwrap());
    m.insert("de".to_string(), serde_json::from_str(include_str!("../locales/de.json")).unwrap());
    m
}

fn format_msg(template: &str, args: &[&str]) -> String {
    let mut result = template.to_string();
    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, arg);
    }
    result
}

pub fn get_message(key: &MessageKey, locale: &str) -> String {
    let locales = LOCALES.get_or_init(load_locales);
    
    let loc = locale.to_lowercase();
    
    let locale_key = if loc.starts_with("ja") {
        "ja"
    } else if loc.starts_with("zh-cn") || loc.starts_with("zh-hans") {
        "zh-cn"
    } else if loc.starts_with("zh-tw") || loc.starts_with("zh-hk") || loc.starts_with("zh-hant") {
        "zh-tw"
    } else if loc.starts_with("ko") {
        "ko"
    } else if loc.starts_with("et") {
        "et"
    } else if loc.starts_with("vi") {
        "vi"
    } else if loc.starts_with("es") {
        "es"
    } else if loc.starts_with("fr") {
        "fr"
    } else if loc.starts_with("de") {
        "de"
    } else {
        "en"
    };

    let lang_map = locales.get(locale_key).unwrap_or_else(|| locales.get("en").unwrap());

    let (template_key, args_vec) = match key {
        MessageKey::MissingInCode(name) => ("missing_in_code", vec![name.clone()]),
        MessageKey::KindMismatch(name, spec_k, code_k) => ("kind_mismatch", vec![name.clone(), spec_k.clone(), code_k.clone()]),
        MessageKey::TypeMismatch(func, param, spec_t, code_t) => ("type_mismatch", vec![func.clone(), param.clone(), spec_t.clone(), code_t.clone()]),
        MessageKey::VarTypeMismatch(name, spec_t, code_t) => ("var_type_mismatch", vec![name.clone(), spec_t.clone(), code_t.clone()]),
        MessageKey::ParamCountMismatch(func, spec_c, code_c) => ("param_count_mismatch", vec![func.clone(), spec_c.to_string(), code_c.to_string()]),
        MessageKey::ReturnTypeMismatch(func, spec_r, code_r) => ("return_type_mismatch", vec![func.clone(), spec_r.clone(), code_r.clone()]),
        MessageKey::LineNumberMissing(name) => ("line_number_missing", vec![name.clone()]),
        MessageKey::LineNumberMismatch(name, spec_l, code_l) => ("line_number_mismatch", vec![name.clone(), spec_l.clone(), code_l.clone()]),
        MessageKey::DependencyNotUsed(caller, callee) => ("dependency_not_used", vec![caller.clone(), callee.clone()]),
        MessageKey::DeadCode(name) => ("dead_code", vec![name.clone()]),
        MessageKey::ReportTitle => ("report_title", vec![]),
        MessageKey::ReportHeader => ("report_header", vec![]),
        MessageKey::ReportSectionTitle => ("report_section_title", vec![]),
        MessageKey::CodeActionTitle(range) => ("code_action_title", vec![range.clone()]),
    };

    let template = lang_map.get(template_key).cloned().unwrap_or_else(|| {
        locales.get("en").unwrap().get(template_key).cloned().unwrap_or_default()
    });

    let args_ref: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
    format_msg(&template, &args_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locales_keys_are_consistent() {
        let locales = load_locales();
        let en_map = locales.get("en").expect("en.json must exist");
        
        for (lang, lang_map) in &locales {
            if lang == "en" {
                continue;
            }
            
            // Check for missing keys in this language compared to en
            for key in en_map.keys() {
                assert!(
                    lang_map.contains_key(key),
                    "Locale '{}' is missing key '{}' defined in en.json",
                    lang,
                    key
                );
            }
            
            // Check for extra keys in this language that are not in en
            for key in lang_map.keys() {
                assert!(
                    en_map.contains_key(key),
                    "Locale '{}' has extra key '{}' not defined in en.json",
                    lang,
                    key
                );
            }
        }
    }
}
