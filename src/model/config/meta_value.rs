//! 模块说明：配置模型定义与序列化反序列化规则。
//!
//! 文件路径：src/model/config/meta_value.rs。
//! 该文件围绕 'meta_value' 的职责提供实现。
//! 关键符号：MetaValue、fmt。

use std::fmt;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::account::amount::Amount;

/// 元数据值类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MetaValue {
    String(String),
    Number(rust_decimal::Decimal),
    Bool(bool),
    Date(NaiveDate),
    Amount(Amount),
}

impl fmt::Display for MetaValue {
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
