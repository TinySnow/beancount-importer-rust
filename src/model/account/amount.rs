//! 金额模块
//!
//! 该模块定义了 `Amount` 结构体，用于表示具有货币单位的金额。
//!
//! # 主要功能
//! - 创建新的金额实例
//! - 金额取反操作
//! - 零值判断
//! - 格式化显示
//!
//! # 关键类型
//! - [`Amount`]：表示金额的核心结构体
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::account::amount::Amount;
//! use rust_decimal::Decimal;
//!
//! // 创建金额实例
//! let amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
//!
//! // 取反操作
//! let negative_amount = amount.negate();
//!
//! // 判断是否为零
//! assert!(!amount.is_zero());
//!
//! // 格式化显示
//! assert_eq!(format!("{}", amount), "100.50 CNY");
//! ```
//!
//! # 注意事项
//! - 货币代码应使用标准的 ISO 4217 货币代码（如 CNY、USD 等）
//! - 对于商品，可使用其代码（如 AAPL、GOOG 等）
//!

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/**
 * 金额：数值 + 货币单位
 *
 * 表示具有特定货币单位的金额，用于财务交易中。
 *
 * # 字段
 * - `number`：金额的数值部分
 * - `currency`：货币或商品代码
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::amount::Amount;
 * use rust_decimal::Decimal;
 *
 * let amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
 * assert_eq!(amount.number, Decimal::from_str_exact("100.50").unwrap());
 * assert_eq!(amount.currency, "CNY");
 * ```
 */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// 金额的数值部分
    pub number: Decimal,
    /// 货币/商品代码（例如：`CNY`、`USD`、`AAPL`）
    pub currency: String,
}

impl Amount {
    /**
     * 创建新的金额实例
     *
     * # 参数
     * - `number`：金额的数值
     * - `currency`：货币或商品代码
     *
     * # 返回值
     * 新创建的 `Amount` 实例
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::amount::Amount;
     * use rust_decimal::Decimal;
     *
     * let amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
     * assert_eq!(amount.number, Decimal::from_str_exact("100.50").unwrap());
     * assert_eq!(amount.currency, "CNY");
     * ```
     */
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
        }
    }

    /**
     * 获取金额的相反数
     *
     * # 返回值
     * 数值取反但货币单位保持不变的新 `Amount` 实例
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::amount::Amount;
     * use rust_decimal::Decimal;
     *
     * let amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
     * let negative_amount = amount.negate();
     * assert_eq!(negative_amount.number, Decimal::from_str_exact("-100.50").unwrap());
     * assert_eq!(negative_amount.currency, "CNY");
     * ```
     */
    pub fn negate(&self) -> Self {
        Self {
            number: -self.number,
            currency: self.currency.clone(),
        }
    }

    /**
     * 判断金额是否为零
     *
     * # 返回值
     * 如果金额数值为零则返回 `true`，否则返回 `false`
     *
     * # 示例
     * ```rust
     * use beancount_importer_rust::model::account::amount::Amount;
     * use rust_decimal::Decimal;
     *
     * let zero_amount = Amount::new(Decimal::from_str_exact("0").unwrap(), "CNY");
     * assert!(zero_amount.is_zero());
     *
     * let non_zero_amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
     * assert!(!non_zero_amount.is_zero());
     * ```
     */
    pub fn is_zero(&self) -> bool {
        self.number.is_zero()
    }
}

/**
 * 为 `Amount` 实现 `Display` trait
 *
 * 格式化显示金额，格式为：`数值 货币代码`
 *
 * # 示例
 * ```rust
 * use beancount_importer_rust::model::account::amount::Amount;
 * use rust_decimal::Decimal;
 *
 * let amount = Amount::new(Decimal::from_str_exact("100.50").unwrap(), "CNY");
 * assert_eq!(format!("{}", amount), "100.50 CNY");
 * ```
 */
impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.currency)
    }
}
