//! 模块说明：规则匹配、条件运算与动作执行引擎。
//!
//! 文件路径：src/model/rule/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：condition、condition_operator、match_mode、match_result。

pub mod condition;
pub mod condition_operator;
pub mod match_mode;
pub mod match_result;
pub mod matcher;
pub mod rule_action;
pub mod rule_def;
pub mod rule_engine;

pub use rule_def::Rule;
