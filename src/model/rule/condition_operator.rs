//! 条件操作符定义。
//!
//! [`ConditionOperator`] 描述规则条件的比较方式，覆盖：
//! - 字符串比较（`equals`、`contains`、`starts_with`、`ends_with`）
//! - 正则匹配（`regex`）
//! - 数值比较（`greater_than`、`less_than`、`between`）
//! - 集合与空值判断（`in`、`not_empty`、`is_empty`）
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::condition_operator::ConditionOperator;
//!
//! let op = ConditionOperator::StartsWith("POS".to_string());
//! match op {
//!     ConditionOperator::StartsWith(prefix) => assert_eq!(prefix, "POS"),
//!     _ => unreachable!("unexpected operator"),
//! }
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};

/// 条件比较操作符。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// 精确匹配（区分大小写）。
    Equals(String),
    /// 子串包含匹配。
    Contains(String),
    /// 正则匹配。
    ///
    /// 序列化时会写为正则表达式字符串，反序列化时重新编译。
    #[serde(with = "serde_regex")]
    Regex(Regex),
    /// 前缀匹配。
    StartsWith(String),
    /// 后缀匹配。
    EndsWith(String),
    /// 数值大于。
    GreaterThan(rust_decimal::Decimal),
    /// 数值小于。
    LessThan(rust_decimal::Decimal),
    /// 数值落在闭区间 `[min, max]` 内。
    Between {
        /// 区间下界（包含）。
        min: rust_decimal::Decimal,
        /// 区间上界（包含）。
        max: rust_decimal::Decimal,
    },
    /// 值位于候选集合中。
    In(Vec<String>),
    /// 非空（字符串长度大于 0）。
    NotEmpty,
    /// 为空（空字符串或字段缺失）。
    IsEmpty,
}

/// 为 [`Regex`] 提供字符串形式的序列化与反序列化。
mod serde_regex {
    use regex::Regex;
    use serde::{self, Deserialize, Deserializer, Serializer};

    /// 将正则表达式写成原始模式字符串。
    pub fn serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(regex.as_str())
    }

    /// 从模式字符串构造 [`Regex`]。
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }
}
