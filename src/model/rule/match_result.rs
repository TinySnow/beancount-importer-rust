//! 规则命中结果聚合模型。
//!
//! [`MatchResult`] 是规则引擎对单条记录执行后的聚合输出：
//! - 标量字段（账户、摘要、标记等）采用“后命中覆盖先命中”策略
//! - 集合字段（标签、链接）采用追加策略
//! - 元数据采用键覆盖合并（后命中同名键覆盖前值）
//! - `ignore` 为累积布尔位，一旦为 `true` 不再回退
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::{
//!     match_result::MatchResult,
//!     rule_action::RuleAction,
//! };
//!
//! let mut result = MatchResult::default();
//! let action = RuleAction {
//!     debit_account: Some("Expenses:Food".to_string()),
//!     tags: vec!["meal".to_string()],
//!     ..Default::default()
//! };
//!
//! result.apply_action(&action);
//! assert_eq!(result.debit_account.as_deref(), Some("Expenses:Food"));
//! assert_eq!(result.tags, vec!["meal"]);
//! ```

use std::collections::HashMap;

use crate::model::rule::rule_action::RuleAction;

/// 对多条命中规则动作进行聚合后的最终结果。
#[derive(Debug, Default)]
pub struct MatchResult {
    /// 借方账户。
    pub debit_account: Option<String>,
    /// 贷方账户。
    pub credit_account: Option<String>,
    /// 手续费账户。
    pub fee_account: Option<String>,
    /// 已实现损益账户。
    pub pnl_account: Option<String>,
    /// 尾差调整账户。
    pub rounding_account: Option<String>,
    /// 交易对手。
    pub payee: Option<String>,
    /// 交易摘要。
    pub narration: Option<String>,
    /// 标签集合（按命中顺序追加）。
    pub tags: Vec<String>,
    /// 链接集合（按命中顺序追加）。
    pub links: Vec<String>,
    /// 交易标记。
    pub flag: Option<char>,
    /// 元数据键值（同名键后值覆盖前值）。
    pub metadata: HashMap<String, String>,
    /// 是否忽略该记录。
    pub ignore: bool,
}

impl MatchResult {
    /// 将一条规则动作合并到当前聚合结果。
    ///
    /// 合并策略：
    /// - `Option<T>` 字段：若动作给出 `Some`，则覆盖当前值。
    /// - `tags` / `links`：直接追加。
    /// - `metadata`：键级覆盖合并。
    /// - `ignore`：逻辑“或”累积。
    ///
    /// # 参数
    /// - `action`：命中的规则动作。
    pub fn apply_action(&mut self, action: &RuleAction) {
        self.apply_action_with_captures(action, &[]);
    }

    /// 将一条规则动作合并到当前聚合结果，支持正则捕获组替换。
    ///
    /// 行为与 [`apply_action`](Self::apply_action) 一致，但在写入标量字符串字段前，
    /// 会将字段值中的 `{1}` / `{2}` / ... 替换为 `captures` 中的对应元素。
    pub fn apply_action_with_captures(&mut self, action: &RuleAction, captures: &[String]) {
        if let Some(ref account) = action.debit_account {
            self.debit_account = Some(substitute_captures(account, captures));
        }
        if let Some(ref account) = action.credit_account {
            self.credit_account = Some(substitute_captures(account, captures));
        }
        if let Some(ref account) = action.fee_account {
            self.fee_account = Some(substitute_captures(account, captures));
        }
        if let Some(ref account) = action.pnl_account {
            self.pnl_account = Some(substitute_captures(account, captures));
        }
        if let Some(ref account) = action.rounding_account {
            self.rounding_account = Some(substitute_captures(account, captures));
        }
        if let Some(ref payee) = action.payee {
            self.payee = Some(substitute_captures(payee, captures));
        }
        if let Some(ref narration) = action.narration {
            self.narration = Some(substitute_captures(narration, captures));
        }
        if let Some(flag) = action.flag {
            self.flag = Some(flag);
        }

        self.tags.extend(action.tags.iter().cloned());
        self.links.extend(action.links.iter().cloned());
        self.metadata.extend(action.metadata.clone());

        if action.ignore {
            self.ignore = true;
        }
    }
}

fn substitute_captures(s: &str, captures: &[String]) -> String {
    if captures.is_empty() {
        return s.to_string();
    }
    let mut result = s.to_string();
    for (i, cap) in captures.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i + 1), cap);
    }
    result
}
