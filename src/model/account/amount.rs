//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
