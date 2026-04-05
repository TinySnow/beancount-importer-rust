//! 元数据值模型。
//!
//! 本模块定义可挂载到交易或过账元数据中的值类型，并提供统一的显示格式。
//! 类型设计对齐 Beancount 元数据常见值：字符串、数值、布尔、日期和金额。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::meta_value::MetaValue;
//! use chrono::NaiveDate;
//! use rust_decimal::Decimal;
//!
//! let date = MetaValue::Date(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
//! let number = MetaValue::Number(Decimal::from_str_exact("12.50").unwrap());
//!
//! assert_eq!(date.to_string(), "2026-04-05");
//! assert_eq!(number.to_string(), "12.50");
//! ```

use std::fmt;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::account::amount::Amount;

/// 元数据值类型。
///
/// 采用 `#[serde(untagged)]` 以便在 YAML/TOML/JSON 中保持简洁写法：
/// 同一个字段可直接反序列化为不同基础类型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MetaValue {
    /// 字符串值，渲染时会自动加双引号。
    String(String),
    /// 数值（高精度十进制）。
    Number(rust_decimal::Decimal),
    /// 布尔值，渲染为 `TRUE` 或 `FALSE`。
    Bool(bool),
    /// 日期值，渲染格式为 `%Y-%m-%d`。
    Date(NaiveDate),
    /// 金额值，沿用 [`Amount`] 的显示格式。
    Amount(Amount),
}

impl fmt::Display for MetaValue {
    /// 按 Beancount 友好的文本格式输出元数据值。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaValue::String(s) => write!(f, "\"{}\"", s),
            MetaValue::Number(n) => write!(f, "{}", n),
            MetaValue::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            MetaValue::Date(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            MetaValue::Amount(a) => write!(f, "{}", a),
        }
    }
}
