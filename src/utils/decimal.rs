//! 模块说明：通用工具函数集合。
//!
//! 文件路径：src/utils/decimal.rs。
//! 该文件围绕 'decimal' 的职责提供实现。
//! 关键符号：parse_decimal、parse_decimal_with_transform、test_simple、test_with_currency。

use rust_decimal::Decimal;
use std::str::FromStr;

/// 解析数值字符串
///
/// 自动处理：
/// - 货币符号 (¥, $, €)
/// - 千分位分隔符 (1,234.56)
/// - 正负号
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

/// 解析并应用转换
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
