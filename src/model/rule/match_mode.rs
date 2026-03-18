//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/match_mode.rs。
//! 该文件围绕 'match_mode' 的职责提供实现。
//! 关键符号：MatchMode。

use serde::{Deserialize, Serialize};

/// 条件组合模式
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// 所有条件都匹配
    #[default]
    And,
    /// 任一条件匹配
    Or,
}
