//! 现金币种规范化工具。
//!
//! 用于将交易明细中的币种文本统一为 Beancount 中常见的大写代码。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::currency::normalize_cash_currency;
//!
//! assert_eq!(normalize_cash_currency(Some("人民币")), "CNY");
//! assert_eq!(normalize_cash_currency(Some("usd")), "USD");
//! assert_eq!(normalize_cash_currency(Some("未知币种")), "CNY");
//! ```

use crate::utils::text::starts_with_ascii_letter;

/// 归一化现金币种文本为 ISO 风格大写代码。
///
/// - 常见中文名称会映射到标准代码；
/// - 对合法 ASCII 代码做大写保留；
/// - 其余或缺失值回退为 `CNY`。
///
/// # 参数
/// - `raw`：原始币种文本，可为空。
///
/// # 返回值
/// 归一化后的币种代码。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::currency::normalize_cash_currency;
///
/// assert_eq!(normalize_cash_currency(Some("港元")), "HKD");
/// assert_eq!(normalize_cash_currency(Some("eur")), "EUR");
/// assert_eq!(normalize_cash_currency(None), "CNY");
/// ```
pub fn normalize_cash_currency(raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or("CNY").trim();
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

#[cfg(test)]
mod tests {
    use super::normalize_cash_currency;

    #[test]
    fn normalizes_common_chinese_currency_labels() {
        assert_eq!(normalize_cash_currency(Some("人民币")), "CNY");
        assert_eq!(normalize_cash_currency(Some("美元")), "USD");
        assert_eq!(normalize_cash_currency(Some("港元")), "HKD");
    }

    #[test]
    fn keeps_valid_ascii_currency_code_uppercase() {
        assert_eq!(normalize_cash_currency(Some("eur")), "EUR");
        assert_eq!(normalize_cash_currency(Some("Usd")), "USD");
    }

    #[test]
    fn falls_back_to_cny_for_invalid_or_missing_value() {
        assert_eq!(normalize_cash_currency(None), "CNY");
        assert_eq!(normalize_cash_currency(Some("")), "CNY");
        assert_eq!(normalize_cash_currency(Some("￥")), "CNY");
    }
}
