//! 规则定义模型。
//!
//! [`Rule`] 是规则系统中的核心配置单元，包含：
//! - 条件列表（[`Condition`]）
//! - 条件组合方式（[`MatchMode`]）
//! - 命中动作（[`RuleAction`]）
//! - 执行控制信息（`priority`、`terminal`）
//!
//! 在规则引擎中，规则会按“优先级 -> 特异度 -> 原始顺序”稳定排序后应用。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::{
//!     Rule,
//!     condition::Condition,
//!     condition_operator::ConditionOperator,
//!     rule_action::RuleAction,
//! };
//!
//! let rule = Rule {
//!     name: Some("coffee".to_string()),
//!     conditions: vec![
//!         Condition {
//!             field: Some("payee".to_string()),
//!             fields: None,
//!             operator: ConditionOperator::Contains("coffee".to_string()),
//!         },
//!         Condition {
//!             field: Some("amount".to_string()),
//!             fields: None,
//!             operator: ConditionOperator::GreaterThan(0.into()),
//!         },
//!     ],
//!     match_mode: Default::default(),
//!     action: RuleAction {
//!         debit_account: Some("Expenses:Food:Coffee".to_string()),
//!         ..Default::default()
//!     },
//!     priority: 10,
//!     terminal: false,
//! };
//!
//! assert_eq!(rule.specificity(), 2);
//! ```

use serde::{Deserialize, Serialize};

use crate::model::rule::{condition::Condition, match_mode::MatchMode, rule_action::RuleAction};

/// 一条可执行匹配规则。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 规则名称（用于调试、日志和追踪）。
    pub name: Option<String>,

    /// 触发该规则所需满足的条件列表。
    pub conditions: Vec<Condition>,

    /// 条件组合模式（`and` / `or`）。
    #[serde(default)]
    pub match_mode: MatchMode,

    /// 规则命中时要应用的动作。
    pub action: RuleAction,

    /// 规则优先级：值越小越先应用，值越大越晚应用。
    #[serde(default)]
    pub priority: i32,

    /// 当前规则命中后是否立即停止后续规则匹配。
    #[serde(default)]
    pub terminal: bool,
}

impl Rule {
    /// 计算规则特异度。
    ///
    /// 当前实现使用条件数量作为特异度近似值：
    /// 条件越多，说明规则越具体。
    pub fn specificity(&self) -> usize {
        self.conditions.len()
    }
}
