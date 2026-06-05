//! 条件匹配器。
//!
//! [`Matcher`] 负责对单条 [`Condition`] 执行实际判定。
//! 该模块同时处理两类字段来源：
//! - `RawRecord` 的标准强类型字段（如 `amount`、`date`）
//! - 供应商扩展字段（`RawRecord::extra`）
//!
//! 对数值比较操作符，匹配器优先读取强类型十进制字段，
//! 并在必要时回退到字符串清洗后解析，提升跨供应商数据兼容性。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     data::raw_record::RawRecord,
//!     rule::{
//!         condition::Condition,
//!         condition_operator::ConditionOperator,
//!         matcher::Matcher,
//!     },
//! };
//! use rust_decimal::Decimal;
//!
//! let mut record = RawRecord::new();
//! record.payee = Some("Coffee Shop".to_string());
//! record.amount = Some(Decimal::from_str_exact("32.50").unwrap());
//!
//! let payee_condition = Condition {
//!     field: "payee".to_string(),
//!     operator: ConditionOperator::Contains("Coffee".to_string()),
//! };
//! let amount_condition = Condition {
//!     field: "amount".to_string(),
//!     operator: ConditionOperator::GreaterThan(Decimal::from_str_exact("20").unwrap()),
//! };
//!
//! assert!(Matcher::matches(&payee_condition, &record));
//! assert!(Matcher::matches(&amount_condition, &record));
//! ```

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
    ///
    /// 对于不存在的字段，大多操作符返回 `false`；
    /// 唯一例外是 [`ConditionOperator::IsEmpty`]，其在字段缺失时返回 `true`。
    pub fn matches(condition: &Condition, record: &RawRecord) -> bool {
        let mut _ignored = Vec::new();
        Self::matches_with_captures(condition, record, &mut _ignored)
    }

    /// 判断一条记录是否命中一个条件，并收集正则捕获组。
    ///
    /// 当操作符为 [`ConditionOperator::Regex`] 且命中时，
    /// 会将所有捕获组（跳过 group 0）按顺序追加到 `captures` 中。
    pub fn matches_with_captures(
        condition: &Condition,
        record: &RawRecord,
        captures: &mut Vec<String>,
    ) -> bool {
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

            ConditionOperator::Regex(regex) => match field_value.as_deref() {
                Some(value) => match regex.captures(value) {
                    Some(caps) => {
                        for cap in caps.iter().skip(1).flatten() {
                            captures.push(cap.as_str().to_string());
                        }
                        true
                    }
                    None => false,
                },
                None => false,
            },

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

    /// 读取字段值并统一成字符串视图。
    ///
    /// 对于标准强类型字段（日期、数值）会按统一格式转换成字符串，
    /// 其余字段通过 `record.get` 走扩展字段回退链路。
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

    /// 将目标字段解析成十进制数值，供数值比较使用。
    ///
    /// 优先读取 `RawRecord` 中已解析的强类型数值字段，
    /// 对其它字段则尝试从字符串回退解析。
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

    /// 从字符串字段中提取并解析十进制数值。
    ///
    /// 实现会过滤掉非数字/符号字符，兼容如 `"CNY -12.30"` 这类输入。
    fn parse_decimal(value: Option<&str>) -> Option<Decimal> {
        value.and_then(|raw| {
            // 保留十进制解析所需字符，尽量容忍上游字段夹带单位或文本。
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
