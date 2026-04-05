//! 价格模块
//!
//! 该模块定义了 `Price` 结构体，用于表示商品或货币的价格信息。
//!
//! # 主要功能
//! - 创建新的价格实例
//! - 格式化显示价格信息
//!
//! # 关键类型
//! - [`Price`]：表示价格信息的核心结构体
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::account::price::Price;
//! use rust_decimal::Decimal;
//!
//! // 创建价格实例
//! let price = Price::new(Decimal::from_str_exact("7.25").unwrap(), "CNY");
//!
//! // 格式化显示
//! assert_eq!(format!("{}", price), "7.25 CNY");
//! ```
//!
//! # 注意事项
//! - 用于记录市场价格、货币转换率等
//! - 通常与 `Amount` 一起使用，表示商品的价值
//!

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/**
 * 价格信息（用于记录市场价格）
 *
 * 表示商品或货币的价格，用于记录市场价格、货币转换率等。
 *
 * # 字段
 * - `number`：单位价格的数值
 * - `currency`：价格的货币单位
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::price::Price;
 * use rust_decimal::Decimal;
 *
 * let price = Price::new(Decimal::from_str_exact("7.25").unwrap(), "CNY");
 * assert_eq!(price.number, Decimal::from_str_exact("7.25").unwrap());
 * assert_eq!(price.currency, "CNY");
 * ```
 */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    /// 单位价格的数值
    pub number: Decimal,
    /// 价格的货币单位
    pub currency: String,
}

impl Price {
    /**
     * 创建新的价格实例
     *
     * # 参数
     * - `number`：单位价格的数值
     * - `currency`：价格的货币单位
     *
     * # 返回值
     * 新创建的 `Price` 实例
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::price::Price;
     * use rust_decimal::Decimal;
     *
     * let price = Price::new(Decimal::from_str_exact("7.25").unwrap(), "CNY");
     * assert_eq!(price.number, Decimal::from_str_exact("7.25").unwrap());
     * assert_eq!(price.currency, "CNY");
     * ```
     */
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
        }
    }
}

/**
 * 为 `Price` 实现 `Display` trait
 *
 * 格式化显示价格信息，格式为：`数值 货币`
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::price::Price;
 * use rust_decimal::Decimal;
 *
 * let price = Price::new(Decimal::from_str_exact("7.25").unwrap(), "CNY");
 * assert_eq!(format!("{}", price), "7.25 CNY");
 * ```
 */
impl fmt::Display for Price {
    /**
     * 格式化价格信息
     *
     * # 参数
     * - `f`：格式化写入器
     *
     * # 返回值
     * 格式化结果
     */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.currency)
    }
}
