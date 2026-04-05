//! 条件组合模式。
//!
//! [`MatchMode`] 用于描述一条规则内部多个条件的组合关系：
//! - `And`：所有条件都命中才算规则命中
//! - `Or`：任一条件命中即可
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::match_mode::MatchMode;
//!
//! assert_eq!(MatchMode::default(), MatchMode::And);
//! assert_ne!(MatchMode::And, MatchMode::Or);
//! ```

use serde::{Deserialize, Serialize};

/// 条件组合模式。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// 所有条件均命中（逻辑与）。
    #[default]
    And,
    /// 任一条件命中（逻辑或）。
    Or,
}
