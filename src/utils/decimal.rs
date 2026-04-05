//! 数值文本解析工具。
//!
//! 提供从交易文本到 [`Decimal`](rust_decimal::Decimal) 的解析逻辑，
//! 并支持常见的一步变换（取反、绝对值）。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::utils::decimal::parse_decimal;
//! use rust_decimal::Decimal;
//!
//! assert_eq!(parse_decimal("$1,234.56"), Some(Decimal::new(123456, 2)));
//! ```

use rust_decimal::Decimal;
use std::str::FromStr;

/// 解析数值字符串。
///
/// 自动处理：
/// - 货币符号 (¥, $, €)
/// - 千分位分隔符 (1,234.56)
/// - 正负号
///
/// # 参数
/// - `s`：待解析文本。
///
/// # 返回值
/// 解析成功返回 `Some(Decimal)`，失败返回 `None`。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::decimal::parse_decimal;
/// use rust_decimal::Decimal;
///
/// assert_eq!(parse_decimal("123.45"), Some(Decimal::new(12345, 2)));
/// assert_eq!(parse_decimal("¥1,234.56"), Some(Decimal::new(123456, 2)));
/// assert_eq!(parse_decimal(""), None);
/// ```
pub fn parse_decimal(s: &str) -> Option<Decimal> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // 移除货币符号和千分位
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();

    if cleaned.is_empty() {
        return None;
    }

    Decimal::from_str(&cleaned).ok()
}

/// 解析数值并应用可选变换。
///
/// 支持的变换：
/// - `"negate"`：取相反数；
/// - `"abs"`：取绝对值；
/// - 其他值或 `None`：保持原值。
///
/// # 参数
/// - `s`：待解析文本。
/// - `transform`：可选变换名称。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::utils::decimal::parse_decimal_with_transform;
/// use rust_decimal::Decimal;
///
/// assert_eq!(
///     parse_decimal_with_transform("123.45", Some("negate")),
///     Some(Decimal::new(-12345, 2))
/// );
/// assert_eq!(
///     parse_decimal_with_transform("-123.45", Some("abs")),
///     Some(Decimal::new(12345, 2))
/// );
/// ```
pub fn parse_decimal_with_transform(s: &str, transform: Option<&str>) -> Option<Decimal> {
    let value = parse_decimal(s)?;

    Some(match transform {
        Some("negate") => -value,
        Some("abs") => value.abs(),
        _ => value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_simple() {
        assert_eq!(parse_decimal("123.45"), Some(dec!(123.45)));
    }

    #[test]
    fn test_with_currency() {
        assert_eq!(parse_decimal("¥123.45"), Some(dec!(123.45)));
        assert_eq!(parse_decimal("$1,234.56"), Some(dec!(1234.56)));
    }

    #[test]
    fn test_negative() {
        assert_eq!(parse_decimal("-123.45"), Some(dec!(-123.45)));
    }

    #[test]
    fn test_transform() {
        assert_eq!(
            parse_decimal_with_transform("123.45", Some("negate")),
            Some(dec!(-123.45))
        );
    }
}
