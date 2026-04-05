//! 规则动作模型。
//!
//! [`RuleAction`] 描述某条规则命中后应产生的变更意图，
//! 由 [`crate::model::rule::match_result::MatchResult`] 负责合并执行。
//!
//! 设计上将动作表达为“可选覆盖 + 可追加集合”的结构：
//! - `Option<T>` 字段用于覆盖目标值
//! - `tags` / `links` / `metadata` 用于累积附加信息
//! - `ignore` 用于标记记录应被过滤
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::rule_action::RuleAction;
//!
//! let action = RuleAction {
//!     debit_account: Some("Expenses:Food".to_string()),
//!     tags: vec!["meal".to_string()],
//!     ignore: false,
//!     ..Default::default()
//! };
//!
//! assert_eq!(action.debit_account.as_deref(), Some("Expenses:Food"));
//! assert_eq!(action.tags, vec!["meal"]);
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 规则命中后要执行的动作集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleAction {
    /// 设置借方账户（常用于费用或资产增加）。
    pub debit_account: Option<String>,

    /// 设置贷方账户（常用于资产减少或收入确认）。
    pub credit_account: Option<String>,

    /// 设置手续费账户（覆盖默认手续费账户）。
    pub fee_account: Option<String>,

    /// 设置已实现损益账户（覆盖默认损益账户）。
    pub pnl_account: Option<String>,

    /// 设置尾差账户（覆盖默认尾差账户）。
    pub rounding_account: Option<String>,

    /// 设置交易对手。
    pub payee: Option<String>,

    /// 设置或追加交易摘要。
    pub narration: Option<String>,

    /// 要追加的标签。
    #[serde(default)]
    pub tags: Vec<String>,

    /// 要追加的链接。
    #[serde(default)]
    pub links: Vec<String>,

    /// 设置交易标记（例如 `*`、`!`）。
    pub flag: Option<char>,

    /// 要合并的元数据键值。
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// 是否忽略该交易记录。
    #[serde(default)]
    pub ignore: bool,
}
