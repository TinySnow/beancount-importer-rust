//! 条件模型。
//!
//! [`Condition`] 表示一条规则中的单个条件，语义为：
//! “对字段 `field` 应用某个 [`ConditionOperator`] 并判定是否命中”。
//!
//! 反序列化时通过 `#[serde(flatten)]` 将操作符结构展开到同层级，
//! 使配置文件无需额外嵌套 `operator` 对象。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::rule::{
//!     condition::Condition,
//!     condition_operator::ConditionOperator,
//! };
//!
//! let condition = Condition {
//!     field: "payee".to_string(),
//!     operator: ConditionOperator::Contains("Coffee".to_string()),
//! };
//!
//! assert_eq!(condition.field, "payee");
//! ```

use serde::{Deserialize, Serialize};

use crate::model::rule::condition_operator::ConditionOperator;

/// 单个匹配条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// 要读取并参与比较的字段名（例如：`payee`、`amount`、`type`）。
    pub field: String,

    /// 匹配操作符与期望值。
    #[serde(flatten)]
    pub operator: ConditionOperator,
}
