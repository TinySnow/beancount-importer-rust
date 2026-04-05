//! 规则系统模型。
//!
//! 本模块定义导入规则的核心数据结构与执行组件，涵盖：
//! - 条件表达（[`condition::Condition`] 与 [`condition_operator::ConditionOperator`]）
//! - 条件组合方式（[`match_mode::MatchMode`]）
//! - 规则动作与结果聚合（[`rule_action::RuleAction`]、[`match_result::MatchResult`]）
//! - 匹配与执行引擎（[`matcher::Matcher`]、[`rule_engine::RuleEngine`]）
//!
//! 典型流程：
//! 1. 构造 [`Rule`]（包含条件、匹配模式、动作、优先级）。
//! 2. 通过 [`rule_engine::RuleEngine`] 对原始记录执行匹配。
//! 3. 得到聚合后的 [`match_result::MatchResult`]，供后续记账映射阶段使用。
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
//! let provider_rule = Rule {
//!     name: Some("coffee".to_string()),
//!     conditions: vec![Condition {
//!         field: "payee".to_string(),
//!         operator: ConditionOperator::Contains("Coffee".to_string()),
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
//! let provider_rules = vec![provider_rule];
//! let global_config = GlobalConfig::default();
//! let engine = RuleEngine::new(&provider_rules, &global_config);
//!
//! let mut record = RawRecord::new();
//! record.payee = Some("Nice Coffee".to_string());
//!
//! let result = engine.match_record(&record);
//! assert_eq!(result.debit_account.as_deref(), Some("Expenses:Food:Coffee"));
//! ```

pub mod condition;
pub mod condition_operator;
pub mod match_mode;
pub mod match_result;
pub mod matcher;
pub mod rule_action;
pub mod rule_def;
pub mod rule_engine;

pub use rule_def::Rule;
