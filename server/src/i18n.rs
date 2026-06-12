pub enum MessageKey {
    MissingInCode(String),
    KindMismatch(String, String, String),
    TypeMismatch(String, String, String, String),
    VarTypeMismatch(String, String, String),
    ParamCountMismatch(String, usize, usize),
    ReturnTypeMismatch(String, String, String),
    LineNumberMissing(String),
    LineNumberMismatch(String, String, String),
    ReportTitle,
    ReportHeader,
    ReportSectionTitle,
    CodeActionTitle(String),
}

pub fn get_message(key: &MessageKey, locale: &str) -> String {
    let loc = locale.to_lowercase();
    
    // Determine language match
    let is_ja = loc.starts_with("ja");
    let is_zh_cn = loc.starts_with("zh-cn") || loc.starts_with("zh-hans");
    let is_zh_tw = loc.starts_with("zh-tw") || loc.starts_with("zh-hk") || loc.starts_with("zh-hant");
    let is_ko = loc.starts_with("ko");
    let is_et = loc.starts_with("et");
    let is_vi = loc.starts_with("vi");
    let is_es = loc.starts_with("es");
    let is_fr = loc.starts_with("fr");
    let is_de = loc.starts_with("de");

    match key {
        MessageKey::MissingInCode(name) => {
            if is_ja {
                format!("仕様書に記載されているシンボル '{}' がコード内に見つかりません。", name)
            } else if is_zh_cn {
                format!("规范中记录的符号 '{}' 在代码中缺失。", name)
            } else if is_zh_tw {
                format!("規範中記錄的符號 '{}' 在程式碼中缺失。", name)
            } else if is_ko {
                format!("명세서에 기록된 '{}' 심볼이 코드에서 누락되었습니다.", name)
            } else if is_et {
                format!("Spetsifikatsioonis märgitud sümbol '{}' puudub koodist.", name)
            } else if is_vi {
                format!("Ký hiệu '{}' được mô tả trong đặc tả bị thiếu trong mã nguồn.", name)
            } else if is_es {
                format!("Falta en el código el símbolo '{}' documentado en las especificaciones.", name)
            } else if is_fr {
                format!("Le symbole '{}' documenté dans les spécifications est manquant dans le code.", name)
            } else if is_de {
                format!("Das in den Spezifikationen dokumentierte Symbol '{}' fehlt im Code.", name)
            } else {
                format!("Symbol '{}' documented in specifications is missing in the code.", name)
            }
        }
        MessageKey::KindMismatch(name, spec_k, code_k) => {
            if is_ja {
                format!("シンボル '{}' の種類が一致しません。仕様書: {}, コード: {}", name, spec_k, code_k)
            } else if is_zh_cn {
                format!("符号 '{}' 的类型不匹配。规范: {}, 代码: {}", name, spec_k, code_k)
            } else if is_zh_tw {
                format!("符號 '{}' 的類型不匹配。規範: {}, 程式碼: {}", name, spec_k, code_k)
            } else if is_ko {
                format!("심볼 '{}'의 종류가 일치하지 않습니다. 명세서: {}, 코드: {}", name, spec_k, code_k)
            } else if is_et {
                format!("Sümboli '{}' tüüp ei kattu. Spetsifikatsioon: {}, Kood: {}", name, spec_k, code_k)
            } else if is_vi {
                format!("Loại của ký hiệu '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", name, spec_k, code_k)
            } else if is_es {
                format!("El tipo de símbolo '{}' no coincide. Espec: {}, Código: {}", name, spec_k, code_k)
            } else if is_fr {
                format!("Le type de symbole '{}' ne correspond pas. Spéc: {}, Code: {}", name, spec_k, code_k)
            } else if is_de {
                format!("Symboltyp für '{}' stimmt nicht überein. Spezifikation: {}, Code: {}", name, spec_k, code_k)
            } else {
                format!("Symbol kind mismatch for '{}'. Spec: {}, Code: {}", name, spec_k, code_k)
            }
        }
        MessageKey::TypeMismatch(func, param, spec_t, code_t) => {
            if is_ja {
                format!("関数 '{}' の引数 '{}' の型が一致しません。仕様書: {}, コード: {}", func, param, spec_t, code_t)
            } else if is_zh_cn {
                format!("函数 '{}' 中的参数 '{}' 类型不匹配。规范: {}, 代码: {}", func, param, spec_t, code_t)
            } else if is_zh_tw {
                format!("函式 '{}' 中的參數 '{}' 型態不匹配。規範: {}, 程式碼: {}", func, param, spec_t, code_t)
            } else if is_ko {
                format!("함수 '{}'의 매개변수 '{}' 타입이 일치하지 않습니다. 명세서: {}, 코드: {}", func, param, spec_t, code_t)
            } else if is_et {
                format!("Parameetri '{}' tüüpide lahknevus funktsioonis '{}'. Spetsifikatsioon: {}, Kood: {}", param, func, spec_t, code_t)
            } else if is_vi {
                format!("Kiểu dữ liệu của tham số '{}' trong hàm '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", param, func, spec_t, code_t)
            } else if is_es {
                format!("Discrepancia de tipo para el parámetro '{}' en la función '{}'. Espec: {}, Código: {}", param, func, spec_t, code_t)
            } else if is_fr {
                format!("Incompatibilité de type pour le paramètre '{}' dans la fonction '{}'. Spéc: {}, Code: {}", param, func, spec_t, code_t)
            } else if is_de {
                format!("Typkonflikt für Parameter '{}' in Funktion '{}'. Spezifikation: {}, Code: {}", param, func, spec_t, code_t)
            } else {
                format!("Type mismatch for parameter '{}' in function '{}'. Spec: {}, Code: {}", param, func, spec_t, code_t)
            }
        }
        MessageKey::VarTypeMismatch(name, spec_t, code_t) => {
            if is_ja {
                format!("変数 '{}' の型が一致しません。仕様書: {}, コード: {}", name, spec_t, code_t)
            } else if is_zh_cn {
                format!("变量 '{}' 的类型不匹配。规范: {}, 代码: {}", name, spec_t, code_t)
            } else if is_zh_tw {
                format!("變數 '{}' 的型態不匹配。規範: {}, 程式碼: {}", name, spec_t, code_t)
            } else if is_ko {
                format!("변수 '{}'의 타입이 일치하지 않습니다. 명세서: {}, 코드: {}", name, spec_t, code_t)
            } else if is_et {
                format!("Muutuja '{}' tüüpide lahknevus. Spetsifikatsioon: {}, Kood: {}", name, spec_t, code_t)
            } else if is_vi {
                format!("Kiểu dữ liệu của biến '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", name, spec_t, code_t)
            } else if is_es {
                format!("Discrepancia de tipo para la variable '{}'. Espec: {}, Código: {}", name, spec_t, code_t)
            } else if is_fr {
                format!("Incompatibilité de type pour la variable '{}'. Spéc: {}, Code: {}", name, spec_t, code_t)
            } else if is_de {
                format!("Typkonflikt für Variable '{}'. Spezifikation: {}, Code: {}", name, spec_t, code_t)
            } else {
                format!("Type mismatch for variable '{}'. Spec: {}, Code: {}", name, spec_t, code_t)
            }
        }
        MessageKey::ParamCountMismatch(func, spec_c, code_c) => {
            if is_ja {
                format!("関数 '{}' の引数の数が一致しません。仕様書: {}, コード: {}", func, spec_c, code_c)
            } else if is_zh_cn {
                format!("函数 '{}' 的参数数量不匹配。规范: {}, 代码: {}", func, spec_c, code_c)
            } else if is_zh_tw {
                format!("函式 '{}' 的參數數量不匹配。規範: {}, 程式碼: {}", func, spec_c, code_c)
            } else if is_ko {
                format!("함수 '{}'의 매개변수 개수가 일치하지 않습니다. 명세서: {}, 코드: {}", func, spec_c, code_c)
            } else if is_et {
                format!("Parameetrite arvu lahknevus funktsioonis '{}'. Spetsifikatsioon: {}, Kood: {}", func, spec_c, code_c)
            } else if is_vi {
                format!("Số lượng tham số của hàm '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", func, spec_c, code_c)
            } else if is_es {
                format!("Discrepancia en el número de parámetros para la función '{}'. Espec: {}, Código: {}", func, spec_c, code_c)
            } else if is_fr {
                format!("Incompatibilité du nombre de paramètres pour la fonction '{}'. Spéc: {}, Code: {}", func, spec_c, code_c)
            } else if is_de {
                format!("Parameteranzahl-Konflikt für Funktion '{}'. Spezifikation: {}, Code: {}", func, spec_c, code_c)
            } else {
                format!("Parameter count mismatch for function '{}'. Spec: {}, Code: {}", func, spec_c, code_c)
            }
        }
        MessageKey::ReturnTypeMismatch(func, spec_r, code_r) => {
            if is_ja {
                format!("関数 '{}' の戻り値の型が一致しません。仕様書: {}, コード: {}", func, spec_r, code_r)
            } else if is_zh_cn {
                format!("函数 '{}' 的返回值类型不匹配。规范: {}, 代码: {}", func, spec_r, code_r)
            } else if is_zh_tw {
                format!("函式 '{}' 的傳回值型態不匹配。規範: {}, 程式碼: {}", func, spec_r, code_r)
            } else if is_ko {
                format!("함수 '{}'의 반환 타입이 일치하지 않습니다. 명세서: {}, 코드: {}", func, spec_r, code_r)
            } else if is_et {
                format!("Tagastustüübi lahknevus funktsioonis '{}'. Spetsifikatsioon: {}, Kood: {}", func, spec_r, code_r)
            } else if is_vi {
                format!("Kiểu trả về của hàm '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", func, spec_r, code_r)
            } else if is_es {
                format!("Discrepancia en el tipo de retorno para la función '{}'. Espec: {}, Código: {}", func, spec_r, code_r)
            } else if is_fr {
                format!("Incompatibilité du type de retour pour la fonction '{}'. Spéc: {}, Code: {}", func, spec_r, code_r)
            } else if is_de {
                format!("Rückgabetyp-Konflikt für Funktion '{}'. Spezifikation: {}, Code: {}", func, spec_r, code_r)
            } else {
                format!("Return type mismatch for function '{}'. Spec: {}, Code: {}", func, spec_r, code_r)
            }
        }
        MessageKey::LineNumberMissing(name) => {
            if is_ja {
                format!("シンボル '{}' の行番号が仕様書に記載されていません。", name)
            } else if is_zh_cn {
                format!("规范中缺少符号 '{}' 的行号。", name)
            } else if is_zh_tw {
                format!("規範中缺少符號 '{}' 的行號。", name)
            } else if is_ko {
                format!("명세서에 심볼 '{}'의 라인 번호가 누락되었습니다.", name)
            } else if is_et {
                format!("Sümboli '{}' reanumber puudub spetsifikatsioonist.", name)
            } else if is_vi {
                format!("Thiếu số dòng cho ký hiệu '{}' trong đặc tả.", name)
            } else if is_es {
                format!("Falta el número de línea para el símbolo '{}' en la especificación.", name)
            } else if is_fr {
                format!("Le numéro de ligne pour le symbole '{}' est manquant dans la spécification.", name)
            } else if is_de {
                format!("Zeilennummer für Symbol '{}' fehlt in der Spezifikation.", name)
            } else {
                format!("Line number for symbol '{}' is missing in the specification.", name)
            }
        }
        MessageKey::LineNumberMismatch(name, spec_l, code_l) => {
            if is_ja {
                format!("シンボル '{}' の行番号が一致しません。仕様書: {}, コード: {}", name, spec_l, code_l)
            } else if is_zh_cn {
                format!("符号 '{}' 的行号不匹配。规范: {}, 代码: {}", name, spec_l, code_l)
            } else if is_zh_tw {
                format!("符號 '{}' 的行號不匹配。規範: {}, 程式碼: {}", name, spec_l, code_l)
            } else if is_ko {
                format!("심볼 '{}'의 라인 번호가 일치하지 않습니다. 명세서: {}, 코드: {}", name, spec_l, code_l)
            } else if is_et {
                format!("Sümboli '{}' reanumbrite lahknevus. Spetsifikatsioon: {}, Kood: {}", name, spec_l, code_l)
            } else if is_vi {
                format!("Số dòng của ký hiệu '{}' không khớp. Đặc tả: {}, Mã nguồn: {}", name, spec_l, code_l)
            } else if is_es {
                format!("Discrepancia en el número de línea para el símbolo '{}'. Espec: {}, Código: {}", name, spec_l, code_l)
            } else if is_fr {
                format!("Incompatibilité de numéro de ligne pour le symbole '{}'. Spéc: {}, Code: {}", name, spec_l, code_l)
            } else if is_de {
                format!("Zeilennummern-Konflikt für Symbol '{}'. Spezifikation: {}, Code: {}", name, spec_l, code_l)
            } else {
                format!("Line number mismatch for symbol '{}'. Spec: {}, Code: {}", name, spec_l, code_l)
            }
        }
        MessageKey::ReportTitle => {
            if is_ja {
                "# 整合性監査レポート (Docs Auditor)\n\n".to_string()
            } else if is_zh_cn {
                "# 一致性审计报告 (Docs Auditor)\n\n".to_string()
            } else if is_zh_tw {
                "# 一致性審計報告 (Docs Auditor)\n\n".to_string()
            } else if is_ko {
                "# 일관성 감사 보고서 (Docs Auditor)\n\n".to_string()
            } else if is_et {
                "# Kooskõla auditi aruanne (Docs Auditor)\n\n".to_string()
            } else if is_vi {
                "# Báo cáo Kiểm toán Tính nhất quán (Docs Auditor)\n\n".to_string()
            } else if is_es {
                "# Informe de auditoría de coherencia (Docs Auditor)\n\n".to_string()
            } else if is_fr {
                "# Rapport d'audit de cohérence (Docs Auditor)\n\n".to_string()
            } else if is_de {
                "# Konsistenzprüfungsbericht (Docs Auditor)\n\n".to_string()
            } else {
                "# Consistency Audit Report (Docs Auditor)\n\n".to_string()
            }
        }
        MessageKey::ReportHeader => {
            if is_ja {
                "仕様書とコードの整合性検査で以下の不一致が検出されました。各項目を修正してください。\n\n".to_string()
            } else if is_zh_cn {
                "检测到规范与代码之间存在以下不一致。请修正每个项目。\n\n".to_string()
            } else if is_zh_tw {
                "偵測到規範與程式碼之間存在以下不一致。請修正每個項目。\n\n".to_string()
            } else if is_ko {
                "명세서와 코드 간의 일치하지 않는 항목들이 감지되었습니다. 각 항목을 수정해 주세요.\n\n".to_string()
            } else if is_et {
                "Tuvastati järgmised lahknevused spetsifikatsioonide ja koodi vahel. Palun parandage iga punkt.\n\n".to_string()
            } else if is_vi {
                "Phát hiện các điểm không nhất quán sau đây giữa đặc tả và mã nguồn. Vui lòng sửa lại từng mục.\n\n".to_string()
            } else if is_es {
                "Se detectaron las siguientes incoherencias entre las especificaciones y el código. Por favor, corrija cada elemento.\n\n".to_string()
            } else if is_fr {
                "Les incohérences suivantes entre les spécifications et le code ont été détectées. Veuillez corriger chaque élément.\n\n".to_string()
            } else if is_de {
                "Die folgenden Unstimmigkeiten zwischen den Spezifikationen und dem Code wurden festgestellt. Bitte korrigieren Sie jedes Element.\n\n".to_string()
            } else {
                "The following inconsistencies between the specifications and the code were detected. Please correct each item.\n\n".to_string()
            }
        }
        MessageKey::ReportSectionTitle => {
            if is_ja {
                "## 不一致項目 (TODO)\n\n".to_string()
            } else if is_zh_cn {
                "## 不一致项目 (TODO)\n\n".to_string()
            } else if is_zh_tw {
                "## 不一致項目 (TODO)\n\n".to_string()
            } else if is_ko {
                "## 일치하지 않는 항목 (TODO)\n\n".to_string()
            } else if is_et {
                "## Lahknevused (TODO)\n\n".to_string()
            } else if is_vi {
                "## Các mục không nhất quán (TODO)\n\n".to_string()
            } else if is_es {
                "## Elementos incoherentes (TODO)\n\n".to_string()
            } else if is_fr {
                "## Éléments incohérents (TODO)\n\n".to_string()
            } else if is_de {
                "## Unstimmige Elemente (TODO)\n\n".to_string()
            } else {
                "## Inconsistent Items (TODO)\n\n".to_string()
            }
        }
        MessageKey::CodeActionTitle(range) => {
            if is_ja {
                format!("行番号 {} を仕様書に自動追記する", range)
            } else if is_zh_cn {
                format!("自动向规范追加行号 {}", range)
            } else if is_zh_tw {
                format!("自動向規範追加行號 {}", range)
            } else if is_ko {
                format!("명세서에 라인 번호 {}를 자동으로 추가", range)
            } else if is_et {
                format!("Lisa reanumbrid {} automaatselt spetsifikatsioonile", range)
            } else if is_vi {
                format!("Tự động thêm số dòng {} vào đặc tả", range)
            } else if is_es {
                format!("Agregar automáticamente los números de línea {} a la especificación", range)
            } else if is_fr {
                format!("Ajouter automatiquement les numéros de ligne {} à la spécification", range)
            } else if is_de {
                format!("Zeilennummern {} automatisch an Spezifikation anhängen", range)
            } else {
                format!("Automatically append line numbers {} to specification", range)
            }
        }
    }
}
