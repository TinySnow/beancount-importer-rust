//! 模块说明：跨 Provider 的证券交易分类、账户规划与分录构建能力。
//!
//! 文件路径：src/providers/shared/securities/normalize.rs。
//! 该文件围绕 'normalize' 的职责提供实现。
//! 关键符号：sanitize_token、starts_with_ascii_letter、normalizes_chinese_currency_to_iso_code、prefixes_numeric_code_with_uppercase_sec_prefix。

/// Normalizes cash currency labels to ISO uppercase codes.
///
/// Falls back to `CNY` when value is missing or invalid, so Beancount
/// output remains parseable.
pub(super) fn normalize_cash_currency(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "CNY".to_string();
    }

    match trimmed {
        "人民币" | "人民币元" | "RMB" | "CNY" => return "CNY".to_string(),
        "美元" | "USD" => return "USD".to_string(),
        "港币" | "港元" | "HKD" => return "HKD".to_string(),
        "欧元" | "EUR" => return "EUR".to_string(),
        "英镑" | "GBP" => return "GBP".to_string(),
        "日元" | "JPY" => return "JPY".to_string(),
        _ => {}
    }

    let upper = trimmed.to_ascii_uppercase();
    if starts_with_ascii_letter(&upper)
        && upper
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
    {
        return upper;
    }

    "CNY".to_string()
}

/// Normalizes security symbol to a valid Beancount commodity.
///
/// Rules:
/// - If raw symbol starts with ASCII letter: keep sanitized uppercase token.
/// - Otherwise: always prefix with `SEC_`.
///
/// `transaction_type` and `security_name` are kept for API compatibility.
pub(super) fn normalize_security_commodity(
    raw_symbol: &str,
    _transaction_type: Option<&str>,
    _security_name: Option<&str>,
) -> String {
    let token = sanitize_token(raw_symbol).to_ascii_uppercase();

    if starts_with_ascii_letter(&token) {
        return token;
    }

    format!("SEC_{}", token)
}

/// Removes characters that are not allowed in commodity symbols.
fn sanitize_token(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        }
    }

    if out.is_empty() {
        "UNKNOWN".to_string()
    } else {
        out
    }
}

/// Returns true if the string starts with an ASCII letter.
fn starts_with_ascii_letter(value: &str) -> bool {
    value
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{normalize_cash_currency, normalize_security_commodity};

    #[test]
    fn normalizes_chinese_currency_to_iso_code() {
        assert_eq!(normalize_cash_currency("人民币"), "CNY");
        assert_eq!(normalize_cash_currency("美元"), "USD");
    }

    #[test]
    fn prefixes_numeric_code_with_uppercase_sec_prefix() {
        let code =
            normalize_security_commodity("161226", Some("开放式基金申购"), Some("国投白银LOF"));
        assert_eq!(code, "SEC_161226");
    }

    #[test]
    fn keeps_alphabetic_symbol_without_sec_prefix() {
        let code = normalize_security_commodity("GC001", Some("融券回购"), Some("GC001"));
        assert_eq!(code, "GC001");
    }
}
