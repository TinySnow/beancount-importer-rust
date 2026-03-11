//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
