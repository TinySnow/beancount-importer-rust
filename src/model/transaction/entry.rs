//! 交易分录模型。
//!
//! 该模块定义 [`Transaction`]，用于表示一条完整的 Beancount `transaction` 指令。
//! 模型支持链式构建交易要素（收款方、标签、链接、过账和元数据），
//! 并提供简化版平衡校验逻辑 [`Transaction::is_balanced`]。
//!
//! # 平衡校验规则
//! - 按币种分别求和，所有币种净额都必须为 0。
//! - 允许至多一个未填写金额的过账；若恰好一个为空，视为可由 Beancount 自动补全。
//! - 若有两个及以上未填写金额的过账，视为不平衡。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::{
//!     account::{amount::Amount, posting::Posting},
//!     transaction::Transaction,
//! };
//! use chrono::NaiveDate;
//! use rust_decimal::Decimal;
//!
//! let tx = Transaction::new(
//!     NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
//!     "Transfer between cash accounts",
//! )
//! .with_posting(
//!     Posting::new("Assets:Cash:Wallet")
//!         .with_amount(Amount::new(Decimal::new(-1_000, 2), "CNY")),
//! )
//! .with_posting(
//!     Posting::new("Assets:Cash:Bank")
//!         .with_amount(Amount::new(Decimal::new(1_000, 2), "CNY")),
//! );
//!
//! assert!(tx.is_balanced());
//! ```

use crate::model::account::posting::Posting;
use crate::model::config::meta_value::MetaValue;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Beancount 交易模型。
///
/// 一笔交易由日期、说明（`narration`）、若干过账（`postings`）以及可选扩展字段组成。
/// 该类型采用 builder 风格 API，便于在解析或规则映射阶段逐步构建对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易日期（`YYYY-MM-DD`）。
    pub date: NaiveDate,
    /// 交易标记（`*` 已确认，`!` 待确认）。
    #[serde(default = "default_flag")]
    pub flag: char,
    /// 交易对手（可选）。
    pub payee: Option<String>,
    /// 交易描述（`narration`）。
    pub narration: String,
    /// 标签列表（不含 `#` 前缀）。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 链接列表（不含 `^` 前缀）。
    #[serde(default)]
    pub links: Vec<String>,
    /// 过账列表，至少应包含两条过账才有实际会计意义。
    pub postings: Vec<Posting>,
    /// 元数据扩展字段。
    #[serde(default)]
    pub metadata: HashMap<String, MetaValue>,
}

/// `flag` 字段的默认值。
///
/// 供 `serde(default = "default_flag")` 在反序列化缺失字段时回填。
fn default_flag() -> char {
    '*'
}

impl Transaction {
    /// 创建新的交易对象。
    ///
    /// 新对象默认使用 `*` 标记，且不含交易对手、标签、链接、过账与元数据。
    ///
    /// # 参数
    /// - `date`：交易日期。
    /// - `narration`：交易说明文本。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::transaction::Transaction;
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(
    ///     NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
    ///     "Monthly salary",
    /// );
    ///
    /// assert_eq!(tx.flag, '*');
    /// assert_eq!(tx.narration, "Monthly salary");
    /// assert!(tx.postings.is_empty());
    /// ```
    pub fn new(date: NaiveDate, narration: impl Into<String>) -> Self {
        Self {
            date,
            flag: default_flag(),
            payee: None,
            narration: narration.into(),
            tags: Vec::new(),
            links: Vec::new(),
            postings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 设置交易对手（`payee`）。
    ///
    /// # 参数
    /// - `payee`：交易对手名称。
    ///
    /// # 返回值
    /// 返回设置后的交易对象（支持链式调用）。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::transaction::Transaction;
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Dinner")
    ///     .with_payee("Local Restaurant");
    ///
    /// assert_eq!(tx.payee.as_deref(), Some("Local Restaurant"));
    /// ```
    pub fn with_payee(mut self, payee: impl Into<String>) -> Self {
        self.payee = Some(payee.into());
        self
    }

    /// 设置交易标记。
    ///
    /// 常见值为 `*`（已确认）或 `!`（待确认）。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::transaction::Transaction;
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Pending payment")
    ///     .with_flag('!');
    ///
    /// assert_eq!(tx.flag, '!');
    /// ```
    pub fn with_flag(mut self, flag: char) -> Self {
        self.flag = flag;
        self
    }

    /// 添加标签。
    ///
    /// # 参数
    /// - `tag`：标签名（不含 `#`）。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::transaction::Transaction;
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Dinner")
    ///     .with_tag("food")
    ///     .with_tag("friends");
    ///
    /// assert_eq!(tx.tags, vec!["food", "friends"]);
    /// ```
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加链接。
    ///
    /// # 参数
    /// - `link`：链接标识（不含 `^`）。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::transaction::Transaction;
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Online order")
    ///     .with_link("order-20260405");
    ///
    /// assert_eq!(tx.links, vec!["order-20260405"]);
    /// ```
    pub fn with_link(mut self, link: impl Into<String>) -> Self {
        self.links.push(link.into());
        self
    }

    /// 添加一条过账。
    ///
    /// # 参数
    /// - `posting`：要加入交易的过账项。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::{
    ///     account::{amount::Amount, posting::Posting},
    ///     transaction::Transaction,
    /// };
    /// use chrono::NaiveDate;
    /// use rust_decimal::Decimal;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Cash withdraw")
    ///     .with_posting(
    ///         Posting::new("Assets:Cash:Wallet")
    ///             .with_amount(Amount::new(Decimal::new(500, 2), "CNY")),
    ///     );
    ///
    /// assert_eq!(tx.postings.len(), 1);
    /// ```
    pub fn with_posting(mut self, posting: Posting) -> Self {
        self.postings.push(posting);
        self
    }

    /// 添加一条元数据。
    ///
    /// 相同 `key` 重复写入时，后值会覆盖前值。
    ///
    /// # 参数
    /// - `key`：元数据键。
    /// - `value`：元数据值。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::{
    ///     config::meta_value::MetaValue,
    ///     transaction::Transaction,
    /// };
    /// use chrono::NaiveDate;
    ///
    /// let tx = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Lunch")
    ///     .with_meta("source", MetaValue::String("importer".to_string()));
    ///
    /// assert!(matches!(
    ///     tx.metadata.get("source"),
    ///     Some(MetaValue::String(v)) if v == "importer"
    /// ));
    /// ```
    pub fn with_meta(mut self, key: impl Into<String>, value: MetaValue) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// 校验交易是否平衡（简化规则）。
    ///
    /// 该实现不做汇率换算，而是按币种分别求和。每个币种总和为 0 视为平衡。
    /// 此外允许一条未填写金额的过账，由 Beancount 在导入时自动推导。
    ///
    /// # 返回值
    /// - `true`：满足简化平衡规则。
    /// - `false`：不满足简化平衡规则。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::{
    ///     account::{amount::Amount, posting::Posting},
    ///     transaction::Transaction,
    /// };
    /// use chrono::NaiveDate;
    /// use rust_decimal::Decimal;
    ///
    /// let balanced = Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Balanced")
    ///     .with_posting(
    ///         Posting::new("Assets:Cash")
    ///             .with_amount(Amount::new(Decimal::new(1_000, 2), "CNY")),
    ///     )
    ///     .with_posting(
    ///         Posting::new("Income:Salary")
    ///             .with_amount(Amount::new(Decimal::new(-1_000, 2), "CNY")),
    ///     );
    /// assert!(balanced.is_balanced());
    ///
    /// let unbalanced =
    ///     Transaction::new(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(), "Unbalanced")
    ///         .with_posting(
    ///             Posting::new("Assets:Cash")
    ///                 .with_amount(Amount::new(Decimal::new(1_000, 2), "CNY")),
    ///         )
    ///         .with_posting(
    ///             Posting::new("Income:Salary")
    ///                 .with_amount(Amount::new(Decimal::new(-900, 2), "CNY")),
    ///         );
    /// assert!(!unbalanced.is_balanced());
    /// ```
    pub fn is_balanced(&self) -> bool {
        // 先按币种累计金额，避免不同币种互相抵消带来的误判。
        let mut balances: HashMap<&str, rust_decimal::Decimal> = HashMap::new();

        for posting in &self.postings {
            if let Some(ref amount) = posting.amount {
                *balances.entry(&amount.currency).or_default() += amount.number;
            }
        }

        // Beancount 仅能自动推导一个缺失金额；超过一个将产生歧义。
        let empty_amount_count = self.postings.iter().filter(|p| p.amount.is_none()).count();

        if empty_amount_count > 1 {
            return false;
        }

        if empty_amount_count == 1 {
            return true;
        }

        // 所有币种净额都为 0 时才平衡。
        balances.values().all(|b| b.is_zero())
    }
}
