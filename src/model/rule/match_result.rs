//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::collections::HashMap;

use crate::model::rule::rule_action::RuleAction;

/// 规则匹配聚合结果。
#[derive(Debug, Default)]
pub struct MatchResult {
    pub debit_account: Option<String>,
    pub credit_account: Option<String>,
    pub fee_account: Option<String>,
    pub pnl_account: Option<String>,
    pub rounding_account: Option<String>,
    pub payee: Option<String>,
    pub narration: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub flag: Option<char>,
    pub metadata: HashMap<String, String>,
    pub ignore: bool,
}

impl MatchResult {
    /// 应用规则动作；后命中的规则覆盖先前结果。
    pub fn apply_action(&mut self, action: &RuleAction) {
        if let Some(ref account) = action.debit_account {
            self.debit_account = Some(account.clone());
        }
        if let Some(ref account) = action.credit_account {
            self.credit_account = Some(account.clone());
        }
        if let Some(ref account) = action.fee_account {
            self.fee_account = Some(account.clone());
        }
        if let Some(ref account) = action.pnl_account {
            self.pnl_account = Some(account.clone());
        }
        if let Some(ref account) = action.rounding_account {
            self.rounding_account = Some(account.clone());
        }
        if let Some(ref payee) = action.payee {
            self.payee = Some(payee.clone());
        }
        if let Some(ref narration) = action.narration {
            self.narration = Some(narration.clone());
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
