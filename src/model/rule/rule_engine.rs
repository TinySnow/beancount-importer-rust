//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/rule_engine.rs。
//! 该文件围绕 'rule_engine' 的职责提供实现。
//! 关键符号：IndexedRule、RuleEngine、new、match_record。

use crate::model::{
    config::global::GlobalConfig,
    data::raw_record::RawRecord,
    rule::{Rule, match_mode::MatchMode, match_result::MatchResult, matcher::Matcher},
};

#[derive(Clone, Copy)]
struct IndexedRule<'a> {
    rule: &'a Rule,
    order: usize,
}

/// 规则引擎：先应用全局规则，再应用供应商规则。
///
/// 匹配策略：
/// 1. 先应用低优先级规则。
/// 2. 先应用低特异度规则。
/// 3. 同级时按文件中先后顺序应用。
/// 4. 后命中的结果覆盖先命中的结果。
pub struct RuleEngine<'a> {
    provider_rules: Vec<IndexedRule<'a>>,
    global_rules: Vec<IndexedRule<'a>>,
}

impl<'a> RuleEngine<'a> {
    /// 构建规则引擎。
    pub fn new(provider_rules: &'a [Rule], global_config: &'a GlobalConfig) -> Self {
        Self {
            provider_rules: Self::prepare_rules(provider_rules),
            global_rules: Self::prepare_rules(&global_config.global_rules),
        }
    }

    /// 匹配一条记录并聚合所有动作。
    pub fn match_record(&self, record: &RawRecord) -> MatchResult {
        // 采用“累积覆盖”策略：先应用低优先级规则，后命中的规则覆盖前值。
        let mut result = MatchResult::default();

        // 全局规则先执行，供应商规则后执行；后者可覆盖前者。
        for indexed in self.global_rules.iter().chain(self.provider_rules.iter()) {
            let rule = indexed.rule;
            // 命中规则后，将动作合并进最终结果。
            if self.rule_matches(rule, record) {
                result.apply_action(&rule.action);

                // `terminal=true` 时立即停止后续规则匹配。
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

    /// 判断一条规则是否命中。
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
