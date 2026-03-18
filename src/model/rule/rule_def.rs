//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/rule_def.rs。
//! 该文件围绕 'rule_def' 的职责提供实现。
//! 关键符号：Rule、specificity。

use serde::{Deserialize, Serialize};

use crate::model::rule::{condition::Condition, match_mode::MatchMode, rule_action::RuleAction};

/// 一条匹配规则。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 规则名称（用于调试）。
    pub name: Option<String>,

    /// 匹配条件列表。
    pub conditions: Vec<Condition>,

    /// 条件组合模式。
    #[serde(default)]
    pub match_mode: MatchMode,

    /// 规则命中时执行的动作。
    pub action: RuleAction,

    /// 规则优先级：值越大越晚应用。
    #[serde(default)]
    pub priority: i32,

    /// 当前规则命中后是否停止后续匹配。
    #[serde(default)]
    pub terminal: bool,
}

impl Rule {
    /// 规则特异度评分：条件越多越具体。
    pub fn specificity(&self) -> usize {
        self.conditions.len()
    }
}
