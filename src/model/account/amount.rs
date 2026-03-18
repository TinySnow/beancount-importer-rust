//! 模块说明：账户与分录相关基础模型（金额、成本、价格、过账项）。
//!
//! 文件路径：src/model/account/amount.rs。
//! 该文件围绕 'amount' 的职责提供实现。
//! 关键符号：Amount、new、negate、is_zero。

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 金额：数值 + 货币单位
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// 数值
    pub number: Decimal,
    /// 货币/商品代码（例如：`CNY`、`USD`、`AAPL`）
    pub currency: String,
}

impl Amount {
    /// 创建新的金额
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
        }
    }

    /// 取反
    pub fn negate(&self) -> Self {
        Self {
            number: -self.number,
            currency: self.currency.clone(),
        }
    }

    /// 判断是否为零
    pub fn is_zero(&self) -> bool {
        self.number.is_zero()
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.currency)
    }
}
