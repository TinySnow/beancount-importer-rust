//! 条件模型。
//!
//! [`Condition`] 表示一条规则中的单个条件，语义为：
//! "对字段 `field`（或 `fields` 中的任一字段）应用某个 [`ConditionOperator`]
//! 并判定是否命中"。
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
//! // 单字段（传统写法）
//! let cond = Condition::new_single("payee", ConditionOperator::Contains("Coffee".to_string()));
//! assert_eq!(cond.field_names(), vec!["payee"]);
//! ```

use serde::{Deserialize, Serialize};

use crate::model::rule::condition_operator::ConditionOperator;

/// 单个匹配条件。
///
/// 配置中可写 `field: "payee"`（单字段）或 `fields: ["peer", "item"]`（多字段任一命中）。
/// 两者不应同时省略；若同时提供则合并为全量字段列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// 要参与比较的单个字段名（与 `fields` 二选一或同时提供）。
    #[serde(default)]
    pub field: Option<String>,

    /// 要参与比较的字段名列表，任一字段命中即满足（与 `field` 二选一或同时提供）。
    #[serde(default)]
    pub fields: Option<Vec<String>>,

    /// 匹配操作符与期望值。
    #[serde(flatten)]
    pub operator: ConditionOperator,
}

impl Condition {
    /// 返回所有待匹配字段名（`fields` 优先，回退到 `field`）。
    pub fn field_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        if let Some(ref field) = self.field {
            names.push(field.as_str());
        }
        if let Some(ref fields) = self.fields {
            for f in fields {
                names.push(f.as_str());
            }
        }
        names
    }

    /// 快速构造单字段条件（测试/文档用）。
    pub fn new_single(field: impl Into<String>, operator: ConditionOperator) -> Self {
        Self {
            field: Some(field.into()),
            fields: None,
            operator,
        }
    }
}
