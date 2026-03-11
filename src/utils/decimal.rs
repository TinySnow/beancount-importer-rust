//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! 数值解析

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
