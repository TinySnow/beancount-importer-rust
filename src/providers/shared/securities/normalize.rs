/// 将现金币种归一化为 ISO 大写代码。
///
/// 如果无法识别，回退为 `CNY`，保证 Beancount 可解析。
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

/// 将证券代码归一化为合法 commodity 符号。
///
/// 规则：
/// - 原始代码以字母开头：直接使用（转大写）。
/// - 数字/其他开头：基金类加 `FUND_`，其余加 `SEC_`。
pub(super) fn normalize_security_commodity(
    raw_symbol: &str,
    transaction_type: Option<&str>,
    security_name: Option<&str>,
) -> String {
    let token = sanitize_token(raw_symbol).to_ascii_uppercase();

    if starts_with_ascii_letter(&token) {
        return token;
    }

    if is_fund_like(transaction_type, security_name) {
        format!("FUND_{}", token)
    } else {
        format!("SEC_{}", token)
    }
}

/// 判断一笔交易是否更接近基金语义。
fn is_fund_like(transaction_type: Option<&str>, security_name: Option<&str>) -> bool {
    let is_fund_text = |text: &str| {
        text.contains("基金")
            || text.contains("LOF")
            || text.contains("lof")
            || text.contains("ETF")
            || text.contains("etf")
            || text.contains("申购")
            || text.contains("赎回")
    };

    transaction_type.map(is_fund_text).unwrap_or(false)
        || security_name.map(is_fund_text).unwrap_or(false)
}

/// 过滤 commodity 中不允许的字符。
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

/// 判断字符串是否以 ASCII 字母开头。
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
    fn prefixes_fund_commodity_with_uppercase_prefix() {
        let code =
            normalize_security_commodity("161226", Some("开放式基金申购"), Some("国投白银LOF"));
        assert_eq!(code, "FUND_161226");
    }

    #[test]
    fn prefixes_non_fund_numeric_code_with_uppercase_sec_prefix() {
        let code = normalize_security_commodity("204001", Some("融券回购"), Some("GC001"));
        assert_eq!(code, "SEC_204001");
    }
}
