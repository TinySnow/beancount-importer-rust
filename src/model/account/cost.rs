//! 成本模块
//!
//! 该模块定义了 `Cost` 结构体，用于表示证券等商品的成本信息。
//!
//! # 主要功能
//! - 创建新的成本实例
//! - 设置成本的购买日期（可选）
//! - 设置成本的批次标签（可选）
//! - 格式化显示成本信息
//!
//! # 关键类型
//! - [`Cost`]：表示成本信息的核心结构体
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::account::cost::Cost;
//! use rust_decimal::Decimal;
//! use chrono::NaiveDate;
//!
//! // 创建基本成本实例
//! let cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD");
//!
//! // 设置日期和标签
//! let cost_with_details = cost
//!     .with_date(NaiveDate::from_ymd(2023, 1, 1))
//!     .with_label("batch-001");
//!
//! // 格式化显示
//! assert_eq!(
//!     format!("{}", cost_with_details),
//!     "100.50 USD, 2023-01-01, \"batch-001\""
//! );
//! ```
//!
//! # 注意事项
//! - 对应 Beancount 的 `{cost}` 语法
//! - 用于追踪证券等商品的成本基础
//!

use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/**
 * 成本信息（用于证券等需要追踪成本基础的商品）
 *
 * 对应 Beancount 的 `{cost}` 语法，用于记录商品的购买成本信息。
 *
 * # 字段
 * - `number`：单位成本的数值
 * - `currency`：成本的货币单位
 * - `date`：购买日期（可选）
 * - `label`：批次标签（可选）
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::cost::Cost;
 * use rust_decimal::Decimal;
 * use chrono::NaiveDate;
 *
 * let cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD")
 *     .with_date(NaiveDate::from_ymd(2023, 1, 1))
 *     .with_label("batch-001");
 *
 * assert_eq!(cost.number, Decimal::from_str_exact("100.50").unwrap());
 * assert_eq!(cost.currency, "USD");
 * assert_eq!(cost.date, Some(NaiveDate::from_ymd(2023, 1, 1)));
 * assert_eq!(cost.label, Some("batch-001".to_string()));
 * ```
 */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// 单位成本的数值
    pub number: Decimal,
    /// 成本的货币单位
    pub currency: String,
    /// 购买日期（可选）
    pub date: Option<NaiveDate>,
    /// 批次标签（可选）
    pub label: Option<String>,
}

impl Cost {
    /**
     * 创建新的成本实例
     *
     * # 参数
     * - `number`：单位成本的数值
     * - `currency`：成本的货币单位
     *
     * # 返回值
     * 新创建的 `Cost` 实例，日期和标签默认为 `None`
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::cost::Cost;
     * use rust_decimal::Decimal;
     *
     * let cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD");
     * assert_eq!(cost.number, Decimal::from_str_exact("100.50").unwrap());
     * assert_eq!(cost.currency, "USD");
     * assert_eq!(cost.date, None);
     * assert_eq!(cost.label, None);
     * ```
     */
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
            date: None,
            label: None,
        }
    }

    /**
     * 设置成本的购买日期
     *
     * # 参数
     * - `date`：购买日期
     *
     * # 返回值
     * 设置了日期的 `Cost` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::cost::Cost;
     * use rust_decimal::Decimal;
     * use chrono::NaiveDate;
     *
     * let cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD")
     *     .with_date(NaiveDate::from_ymd(2023, 1, 1));
     * assert_eq!(cost.date, Some(NaiveDate::from_ymd(2023, 1, 1)));
     * ```
     */
    pub fn with_date(mut self, date: NaiveDate) -> Self {
        self.date = Some(date);
        self
    }

    /**
     * 设置成本的批次标签
     *
     * # 参数
     * - `label`：批次标签
     *
     * # 返回值
     * 设置了标签的 `Cost` 实例（链式调用）
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::cost::Cost;
     * use rust_decimal::Decimal;
     *
     * let cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD")
     *     .with_label("batch-001");
     * assert_eq!(cost.label, Some("batch-001".to_string()));
     * ```
     */
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/**
 * 为 `Cost` 实现 `Display` trait
 *
 * 格式化显示成本信息，格式为：`数值 货币[, 日期][, "标签"]`
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::cost::Cost;
 * use rust_decimal::Decimal;
 * use chrono::NaiveDate;
 *
 * // 基本成本
 * let basic_cost = Cost::new(Decimal::from_str_exact("100.50").unwrap(), "USD");
 * assert_eq!(format!("{}", basic_cost), "100.50 USD");
 *
 * // 带日期的成本
 * let cost_with_date = basic_cost.with_date(NaiveDate::from_ymd(2023, 1, 1));
 * assert_eq!(format!("{}", cost_with_date), "100.50 USD, 2023-01-01");
 *
 * // 带日期和标签的成本
 * let cost_with_details = cost_with_date.with_label("batch-001");
 * assert_eq!(
 *     format!("{}", cost_with_details),
 *     "100.50 USD, 2023-01-01, \"batch-001\""
 * );
 * ```
 */
impl fmt::Display for Cost {
    /**
     * 格式化成本信息
     *
     * # 参数
     * - `f`：格式化写入器
     *
     * # 返回值
     * 格式化结果
     */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.currency)?;
        if let Some(ref date) = self.date {
            write!(f, ", {}", date.format("%Y-%m-%d"))?;
        }
        if let Some(ref label) = self.label {
            write!(f, ", \"{}\"", label)?;
        }
        Ok(())
    }
}
