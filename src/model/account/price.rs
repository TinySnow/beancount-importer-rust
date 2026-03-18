//! 模块说明：账户与分录相关基础模型（金额、成本、价格、过账项）。
//!
//! 文件路径：src/model/account/price.rs。
//! 该文件围绕 'price' 的职责提供实现。
//! 关键符号：Price、new、fmt。

use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 价格信息（用于记录市场价格）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    /// 单位价格
    pub number: Decimal,
    /// 价格货币
    pub currency: String,
}

impl Price {
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
        }
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.currency)
    }
}
