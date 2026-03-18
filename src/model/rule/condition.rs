//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/condition.rs。
//! 该文件围绕 'condition' 的职责提供实现。
//! 关键符号：Condition。

use serde::{Deserialize, Serialize};

use crate::model::rule::condition_operator::ConditionOperator;

/// 匹配条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// 要匹配的字段名
    pub field: String,

    /// 匹配操作符
    #[serde(flatten)]
    pub operator: ConditionOperator,
}
