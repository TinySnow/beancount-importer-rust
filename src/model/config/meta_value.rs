//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
