//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use regex::Regex;
use serde::{Deserialize, Serialize};

/// 条件操作符
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// 精确匹配
    Equals(String),
    /// 包含
    Contains(String),
    /// 正则匹配
    #[serde(with = "serde_regex")]
    Regex(Regex),
    /// 前缀匹配
    StartsWith(String),
    /// 后缀匹配
    EndsWith(String),
    /// 数值大于
    GreaterThan(rust_decimal::Decimal),
    /// 数值小于
    LessThan(rust_decimal::Decimal),
    /// 数值范围
    Between {
        min: rust_decimal::Decimal,
        max: rust_decimal::Decimal,
    },
    /// 在列表中
    In(Vec<String>),
    /// 不为空
    NotEmpty,
    /// 为空
    IsEmpty,
}

/// 用于序列化正则表达式的模块
mod serde_regex {
    use regex::Regex;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(regex.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }
}
