//! 规则执行引擎。
//!
//! [`RuleEngine`] 负责对单条原始记录执行规则匹配并聚合动作结果。
//!
//! 执行顺序与覆盖策略：
//! 1. 全局规则先执行，供应商规则后执行。
//! 2. 同一组内按 `priority` 升序、`specificity` 升序、定义顺序升序执行。
//! 3. 后命中的动作可覆盖先命中的标量字段。
//! 4. 命中 `terminal = true` 的规则后立即停止。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     config::global::GlobalConfig,
//!     data::raw_record::RawRecord,
//!     rule::{
//!         Rule,
//!         condition::Condition,
//!         condition_operator::ConditionOperator,
//!         rule_action::RuleAction,
//!         rule_engine::RuleEngine,
//!     },
//! };
//!
//! let global_rule = Rule {
//!     name: Some("global-coffee".to_string()),
//!     conditions: vec![Condition {
//!         field: "payee".to_string(),
//!         operator: ConditionOperator::Contains("coffee".to_string()),
//!     }],
//!     match_mode: Default::default(),
//!     action: RuleAction {
//!         debit_account: Some("Expenses:Food:Coffee".to_string()),
//!         ..Default::default()
//!     },
//!     priority: 0,
//!     terminal: false,
//! };
//!
//! let provider_rule = Rule {
//!     name: Some("provider-coffee".to_string()),
//!     conditions: vec![Condition {
//!         field: "payee".to_string(),
//!         operator: ConditionOperator::Contains("coffee".to_string()),
//!     }],
//!     match_mode: Default::default(),
//!     action: RuleAction {
//!         debit_account: Some("Expenses:Coffee:Specialty".to_string()),
//!         ..Default::default()
//!     },
//!     priority: 0,
//!     terminal: false,
//! };
//!
//! let mut config = GlobalConfig::default();
//! config.global_rules.push(global_rule);
//! let provider_rules = vec![provider_rule];
//! let engine = RuleEngine::new(&provider_rules, &config);
//!
//! let mut record = RawRecord::new();
//! record.payee = Some("best coffee".to_string());
//!
//! let result = engine.match_record(&record);
//! assert_eq!(
//!     result.debit_account.as_deref(),
//!     Some("Expenses:Coffee:Specialty")
//! );
//! ```

use crate::model::{
    config::global::GlobalConfig,
    data::raw_record::RawRecord,
    rule::{Rule, match_mode::MatchMode, match_result::MatchResult, matcher::Matcher},
};

/// 规则及其原始顺序下标。
///
/// `order` 用于在排序条件完全相同的情况下维持稳定顺序。
#[derive(Clone, Copy)]
struct IndexedRule<'a> {
    /// 规则引用。
    rule: &'a Rule,
    /// 在原始配置中的顺序。
    order: usize,
}

/// 规则引擎：先应用全局规则，再应用供应商规则。
pub struct RuleEngine<'a> {
    /// 已排序的供应商规则。
    provider_rules: Vec<IndexedRule<'a>>,
    /// 已排序的全局规则。
    global_rules: Vec<IndexedRule<'a>>,
}

impl<'a> RuleEngine<'a> {
    /// 构建规则引擎并预处理排序。
    pub fn new(provider_rules: &'a [Rule], global_config: &'a GlobalConfig) -> Self {
        Self {
            provider_rules: Self::prepare_rules(provider_rules),
            global_rules: Self::prepare_rules(&global_config.global_rules),
        }
    }

    /// 匹配一条记录并聚合所有命中动作。
    pub fn match_record(&self, record: &RawRecord) -> MatchResult {
        // 采用“累积覆盖”策略：先应用低优先级规则，后命中的规则覆盖前值。
        let mut result = MatchResult::default();

        // 全局规则先执行，供应商规则后执行；后者可覆盖前者。
        for indexed in self.global_rules.iter().chain(self.provider_rules.iter()) {
            let rule = indexed.rule;
            if self.rule_matches(rule, record) {
                result.apply_action(&rule.action);

                // `terminal = true` 时立即停止后续匹配。
                if rule.terminal {
                    break;
                }
            }
        }

        result
    }

    /// 对规则做稳定排序，确保匹配结果可预测。
    fn prepare_rules(rules: &'a [Rule]) -> Vec<IndexedRule<'a>> {
        let mut indexed_rules: Vec<_> = rules
            .iter()
            .enumerate()
            .map(|(order, rule)| IndexedRule { rule, order })
            .collect();

        indexed_rules.sort_by(|a, b| {
            a.rule
                .priority
                .cmp(&b.rule.priority)
                .then(a.rule.specificity().cmp(&b.rule.specificity()))
                .then(a.order.cmp(&b.order))
        });

        indexed_rules
    }

    /// 判断一条规则是否命中当前记录。
    fn rule_matches(&self, rule: &Rule, record: &RawRecord) -> bool {
        if rule.conditions.is_empty() {
            return false;
        }

        match rule.match_mode {
            MatchMode::And => rule
                .conditions
                .iter()
                .all(|cond| Matcher::matches(cond, record)),
            MatchMode::Or => rule
                .conditions
                .iter()
                .any(|cond| Matcher::matches(cond, record)),
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::model::{
        config::global::GlobalConfig,
        data::raw_record::RawRecord,
        rule::{
            Rule, condition::Condition, condition_operator::ConditionOperator,
            rule_action::RuleAction,
        },
    };

    use super::RuleEngine;

    #[test]
    fn provider_rules_override_global_rules() {
        let global_rule = Rule {
            name: Some("global".to_string()),
            conditions: vec![Condition {
                field: "payee".to_string(),
                operator: ConditionOperator::Contains("coffee".to_string()),
            }],
            match_mode: Default::default(),
            action: RuleAction {
                debit_account: Some("Expenses:Food:Coffee".to_string()),
                ..Default::default()
            },
            priority: 0,
            terminal: false,
        };

        let provider_rule = Rule {
            name: Some("provider".to_string()),
            conditions: vec![Condition {
                field: "payee".to_string(),
                operator: ConditionOperator::Contains("coffee".to_string()),
            }],
            match_mode: Default::default(),
            action: RuleAction {
                debit_account: Some("Expenses:Coffee:Specialty".to_string()),
                ..Default::default()
            },
            priority: 0,
            terminal: false,
        };

        let mut global_config = GlobalConfig::default();
        global_config.global_rules.push(global_rule);
        let provider_rules = [provider_rule];
        let rule_engine = RuleEngine::new(&provider_rules, &global_config);

        let mut record = RawRecord::new();
        record.payee = Some("best coffee".to_string());
        record.amount = Some(dec!(32.5));

        let result = rule_engine.match_record(&record);
        assert_eq!(
            result.debit_account.as_deref(),
            Some("Expenses:Coffee:Specialty")
        );
    }
}
