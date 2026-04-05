//! 证券转换中的币种与标的代码归一化工具。
//!
//! 负责把 Provider 文本字段转换为 Beancount 可解析的稳定符号，
//! 包括现金币种标准化与证券商品代码规范化。

use crate::utils::{
    currency::normalize_cash_currency as normalize_common_cash_currency,
    text::starts_with_ascii_letter,
};

/// 将现金币种标签归一化为 ISO 大写代码。
///
/// 输入异常时回退到 `CNY`，确保生成的 Beancount 分录始终可解析。
pub(super) fn normalize_cash_currency(raw: &str) -> String {
    normalize_common_cash_currency(Some(raw))
}

/// 将证券代码归一化为合法的 Beancount commodity。
///
/// 规则：
/// - 若净化后的代码以 ASCII 字母开头：直接使用大写代码；
/// - 否则统一添加 `SEC_` 前缀，避免以数字开头导致解析歧义。
///
/// `transaction_type` 与 `security_name` 当前保留用于接口兼容。
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

/// 移除 commodity 不允许的字符。
///
/// 仅保留字母、数字、下划线、短横线和点号。
/// 若结果为空，回退为 `UNKNOWN`。
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
