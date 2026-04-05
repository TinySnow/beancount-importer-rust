//! 过账模块
//!
//! 该模块定义了 `Posting` 结构体，用于表示账户变动（过账项）。
//!
//! # 主要功能
//! - 创建新的过账实例
//! - 设置过账金额
//! - 设置成本信息（用于证券等）
//! - 设置推断成本标记
//! - 设置价格信息
//! - 设置过账标记
//! - 添加元数据
//!
//! # 关键类型
//! - [`Posting`]：表示过账项的核心结构体
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::account::posting::Posting;
//! use beancount_importer_rust::model::account::amount::Amount;
//! use beancount_importer_rust::model::account::cost::Cost;
//! use beancount_importer_rust::model::account::price::Price;
//! use beancount_importer_rust::model::config::meta_value::MetaValue;
//! use rust_decimal::Decimal;
//! use chrono::NaiveDate;
//!
//! // 创建基本过账
//! let posting = Posting::new("Assets:Cash")
//!     .with_amount(Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY"))
//!     .with_flag('*')
//!     .with_meta("note", MetaValue::String("Salary deposit".to_string()));
//!
//! // 创建带有成本的过账（用于证券）
//! let stock_posting = Posting::new("Assets:Investments:Stocks")
//!     .with_amount(Amount::new(Decimal::from_str_exact("10").unwrap(), "AAPL"))
//!     .with_cost(Cost::new(Decimal::from_str_exact("150.25").unwrap(), "USD")
//!         .with_date(NaiveDate::from_ymd(2023, 1, 1)))
//!     .with_price(Price::new(Decimal::from_str_exact("160.50").unwrap(), "USD"));
//! ```
//!
//! # 注意事项
//! - 金额字段可选，为空时由 Beancount 自动计算
//! - 成本字段用于证券等需要追踪成本基础的商品
//! - 推断成本标记用于按已有持仓成本自动匹配
//!

use std::collections::HashMap;

use ::serde::{Deserialize, Serialize};

use crate::model::{
    account::{amount::Amount, cost::Cost, price::Price},
    config::meta_value::MetaValue,
};

/**
 * 过账（账户变动）
 *
 * 表示一笔账户变动，是 Beancount 分录（Transaction）的基本组成部分。
 *
 * # 字段
 * - `account`：账户名称
 * - `amount`：金额（可选，为空时由 Beancount 自动计算）
 * - `cost`：成本信息（用于证券等需要追踪成本基础的商品）
 * - `inferred_cost`：标记为 `{}`，用于按已有持仓成本自动匹配
 * - `price`：价格信息（用于货币转换或市值记录）
 * - `flag`：过账标记（可选）
 * - `metadata`：元数据键值对
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::posting::Posting;
 * use beancount_importer_rust::model::account::amount::Amount;
 * use rust_decimal::Decimal;
 *
 * let posting = Posting::new("Assets:Cash")
 *     .with_amount(Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY"));
 *
 * assert_eq!(posting.account, "Assets:Cash");
 * assert!(posting.amount.is_some());
 * assert!(posting.cost.is_none());
 * assert!(!posting.inferred_cost);
 * assert!(posting.price.is_none());
 * assert!(posting.flag.is_none());
 * assert!(posting.metadata.is_empty());
 * ```
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    /// 账户名称（例如："Assets:Cash"、"Expenses:Food"）
    pub account: String,
    /// 金额（可选，为空时由 Beancount 自动计算）
    pub amount: Option<Amount>,
    /// 成本信息（用于证券等需要追踪成本基础的商品）
    pub cost: Option<Cost>,
    /// 标记为 `{}`，用于按已有持仓成本自动匹配
    #[serde(default)]
    pub inferred_cost: bool,
    /// 价格信息（用于货币转换或市值记录）
    pub price: Option<Price>,
    /// 过账标记（可选，例如：'*'、'!'）
    pub flag: Option<char>,
    /// 元数据键值对
    #[serde(default)]
    pub metadata: HashMap<String, MetaValue>,
}

impl Posting {
    /**
     * 创建新的过账实例
     *
     * # 参数
     * - `account`：账户名称
     *
     * # 返回值
     * 新创建的 `Posting` 实例，所有可选字段默认为 `None` 或空
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     *
     * let posting = Posting::new("Assets:Cash");
     * assert_eq!(posting.account, "Assets:Cash");
     * assert!(posting.amount.is_none());
     * assert!(posting.cost.is_none());
     * assert!(!posting.inferred_cost);
     * assert!(posting.price.is_none());
     * assert!(posting.flag.is_none());
     * assert!(posting.metadata.is_empty());
     * ```
     */
    pub fn new(account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            amount: None,
            cost: None,
            inferred_cost: false,
            price: None,
            flag: None,
            metadata: HashMap::new(),
        }
    }

    /**
     * 设置过账金额
     *
     * # 参数
     * - `amount`：金额实例
     *
     * # 返回值
     * 设置了金额的 `Posting` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     * use beancount_importer_rust::model::account::amount::Amount;
     * use rust_decimal::Decimal;
     *
     * let posting = Posting::new("Assets:Cash")
     *     .with_amount(Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY"));
     *
     * assert!(posting.amount.is_some());
     * assert_eq!(posting.amount.unwrap().currency, "CNY");
     * ```
     */
    pub fn with_amount(mut self, amount: Amount) -> Self {
        self.amount = Some(amount);
        self
    }

    /**
     * 设置成本信息
     *
     * # 参数
     * - `cost`：成本实例
     *
     * # 返回值
     * 设置了成本的 `Posting` 实例（链式调用），同时会将 `inferred_cost` 设为 `false`
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     * use beancount_importer_rust::model::account::amount::Amount;
     * use beancount_importer_rust::model::account::cost::Cost;
     * use rust_decimal::Decimal;
     *
     * let posting = Posting::new("Assets:Investments:Stocks")
     *     .with_amount(Amount::new(Decimal::from_str_exact("10").unwrap(), "AAPL"))
     *     .with_cost(Cost::new(Decimal::from_str_exact("150.25").unwrap(), "USD"));
     *
     * assert!(posting.cost.is_some());
     * assert!(!posting.inferred_cost);
     * ```
     */
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self.inferred_cost = false;
        self
    }

    /**
     * 设置推断成本标记
     *
     * 将成本设为 `None` 并将 `inferred_cost` 设为 `true`，表示由 Beancount 按已有持仓成本自动匹配
     *
     * # 返回值
     * 设置了推断成本标记的 `Posting` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     * use beancount_importer_rust::model::account::amount::Amount;
     * use rust_decimal::Decimal;
     *
     * let posting = Posting::new("Assets:Investments:Stocks")
     *     .with_amount(Amount::new(Decimal::from_str_exact("10").unwrap(), "AAPL"))
     *     .with_inferred_cost();
     *
     * assert!(posting.cost.is_none());
     * assert!(posting.inferred_cost);
     * ```
     */
    pub fn with_inferred_cost(mut self) -> Self {
        self.cost = None;
        self.inferred_cost = true;
        self
    }

    /**
     * 设置价格信息
     *
     * # 参数
     * - `price`：价格实例
     *
     * # 返回值
     * 设置了价格的 `Posting` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     * use beancount_importer_rust::model::account::amount::Amount;
     * use beancount_importer_rust::model::account::price::Price;
     * use rust_decimal::Decimal;
     *
     * let posting = Posting::new("Assets:Cash:Foreign")
     *     .with_amount(Amount::new(Decimal::from_str_exact("100").unwrap(), "USD"))
     *     .with_price(Price::new(Decimal::from_str_exact("7.25").unwrap(), "CNY"));
     *
     * assert!(posting.price.is_some());
     * assert_eq!(posting.price.unwrap().currency, "CNY");
     * ```
     */
    pub fn with_price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /**
     * 设置过账标记
     *
     * # 参数
     * - `flag`：过账标记字符（例如：'*'、'!'）
     *
     * # 返回值
     * 设置了标记的 `Posting` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     *
     * let posting = Posting::new("Assets:Cash")
     *     .with_flag('*');
     *
     * assert_eq!(posting.flag, Some('*'));
     * ```
     */
    pub fn with_flag(mut self, flag: char) -> Self {
        self.flag = Some(flag);
        self
    }

    /**
     * 添加元数据
     *
     * # 参数
     * - `key`：元数据键
     * - `value`：元数据值
     *
     * # 返回值
     * 添加了元数据的 `Posting` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::posting::Posting;
     * use beancount_importer_rust::model::config::meta_value::MetaValue;
     *
     * let posting = Posting::new("Assets:Cash")
     *     .with_meta("note", MetaValue::String("Salary deposit".to_string()))
     *     .with_meta("date", MetaValue::Date(chrono::NaiveDate::from_ymd(2023, 1, 1)));
     *
     * assert_eq!(posting.metadata.len(), 2);
     * assert!(posting.metadata.contains_key("note"));
     * ```
     */
    pub fn with_meta(mut self, key: impl Into<String>, value: MetaValue) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}
