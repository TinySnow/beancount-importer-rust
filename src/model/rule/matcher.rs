//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::{borrow::Cow, str::FromStr};

use rust_decimal::Decimal;

use crate::model::{
    data::raw_record::RawRecord,
    rule::{condition::Condition, condition_operator::ConditionOperator},
};

/// 条件匹配器。
pub struct Matcher;

impl Matcher {
    /// 判断一条记录是否命中一个条件。
    pub fn matches(condition: &Condition, record: &RawRecord) -> bool {
        let field_name = condition.field.as_str();
        let field_value = Self::field_value(record, field_name);

        match &condition.operator {
            ConditionOperator::Equals(expected) => field_value
                .as_deref()
                .map(|value| value == expected)
                .unwrap_or(false),

            ConditionOperator::Contains(pattern) => field_value
                .as_deref()
                .map(|value| value.contains(pattern))
                .unwrap_or(false),

            ConditionOperator::Regex(regex) => field_value
                .as_deref()
                .map(|value| regex.is_match(value))
                .unwrap_or(false),

            ConditionOperator::StartsWith(prefix) => field_value
                .as_deref()
                .map(|value| value.starts_with(prefix))
                .unwrap_or(false),

            ConditionOperator::EndsWith(suffix) => field_value
                .as_deref()
                .map(|value| value.ends_with(suffix))
                .unwrap_or(false),

            ConditionOperator::GreaterThan(threshold) => {
                Self::parse_decimal_field(record, field_name, field_value.as_deref())
                    .map(|value| value > *threshold)
                    .unwrap_or(false)
            }

            ConditionOperator::LessThan(threshold) => {
                Self::parse_decimal_field(record, field_name, field_value.as_deref())
                    .map(|value| value < *threshold)
                    .unwrap_or(false)
            }

            ConditionOperator::Between { min, max } => {
                Self::parse_decimal_field(record, field_name, field_value.as_deref())
                    .map(|value| value >= *min && value <= *max)
                    .unwrap_or(false)
            }

            ConditionOperator::In(values) => field_value
                .as_deref()
                .map(|value| values.iter().any(|candidate| candidate == value))
                .unwrap_or(false),

            ConditionOperator::NotEmpty => field_value
                .as_deref()
                .map(|value| !value.is_empty())
                .unwrap_or(false),

            ConditionOperator::IsEmpty => field_value
                .as_deref()
                .map(|value| value.is_empty())
                .unwrap_or(true),
        }
    }

    fn field_value<'a>(record: &'a RawRecord, field_name: &str) -> Option<Cow<'a, str>> {
        match field_name {
            "date" => record
                .date
                .map(|value| Cow::Owned(value.format("%Y-%m-%d").to_string())),
            "amount" => record
                .amount
                .map(|value| Cow::Owned(value.normalize().to_string())),
            "quantity" => record
                .quantity
                .map(|value| Cow::Owned(value.normalize().to_string())),
            "unit_price" => record
                .unit_price
                .map(|value| Cow::Owned(value.normalize().to_string())),
            "fee" => record
                .fee
                .map(|value| Cow::Owned(value.normalize().to_string())),
            "tax" => record
                .tax
                .map(|value| Cow::Owned(value.normalize().to_string())),
            _ => record.get(field_name).map(Cow::Borrowed),
        }
    }

    fn parse_decimal_field(
        record: &RawRecord,
        field_name: &str,
        fallback: Option<&str>,
    ) -> Option<Decimal> {
        match field_name {
            "amount" => record.amount,
            "quantity" => record.quantity,
            "unit_price" => record.unit_price,
            "fee" => record.fee,
            "tax" => record.tax,
            _ => Self::parse_decimal(fallback),
        }
    }

    /// 从字符串字段解析十进制数值。
    fn parse_decimal(value: Option<&str>) -> Option<Decimal> {
        value.and_then(|raw| {
            let cleaned: String = raw
                .chars()
                .filter(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == '-' || *ch == '+')
                .collect();
            Decimal::from_str(&cleaned).ok()
        })
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use super::*;

    fn make_record(payee: &str, amount: Decimal) -> RawRecord {
        let mut record = RawRecord::new();
        record.payee = Some(payee.to_string());
        record.amount = Some(amount);
        record
    }

    #[test]
    fn test_equals_match() {
        let condition = Condition {
            field: "payee".to_string(),
            operator: ConditionOperator::Equals("Starbucks".to_string()),
        };

        assert!(Matcher::matches(
            &condition,
            &make_record("Starbucks", dec!(10.00))
        ));
        assert!(!Matcher::matches(
            &condition,
            &make_record("McDonald's", dec!(10.00))
        ));
    }

    #[test]
    fn test_regex_match() {
        let condition = Condition {
            field: "payee".to_string(),
            operator: ConditionOperator::Regex(Regex::new(r"(?i)coffee").expect("valid regex")),
        };

        assert!(Matcher::matches(
            &condition,
            &make_record("Starbucks Coffee", dec!(10.00))
        ));
        assert!(Matcher::matches(
            &condition,
            &make_record("COFFEE SHOP", dec!(10.00))
        ));
        assert!(!Matcher::matches(
            &condition,
            &make_record("Tea House", dec!(10.00))
        ));
    }

    #[test]
    fn test_greater_than_match() {
        let condition = Condition {
            field: "amount".to_string(),
            operator: ConditionOperator::GreaterThan(Decimal::from(100)),
        };

        assert!(Matcher::matches(
            &condition,
            &make_record("Test", dec!(150.00))
        ));
        assert!(!Matcher::matches(
            &condition,
            &make_record("Test", dec!(50.00))
        ));
    }
}
