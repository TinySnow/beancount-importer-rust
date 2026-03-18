//! 模块说明：账户与分录相关基础模型（金额、成本、价格、过账项）。
//!
//! 文件路径：src/model/account/cost.rs。
//! 该文件围绕 'cost' 的职责提供实现。
//! 关键符号：Cost、new、with_date、with_label。

use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 成本信息（用于证券等需要追踪成本基础的商品）
///
/// 对应 Beancount 的 `{cost}` 语法
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// 单位成本
    pub number: Decimal,
    /// 成本货币
    pub currency: String,
    /// 购买日期（可选）
    pub date: Option<NaiveDate>,
    /// 批次标签（可选）
    pub label: Option<String>,
}

impl Cost {
    /// 创建新的成本
    pub fn new(number: Decimal, currency: impl Into<String>) -> Self {
        Self {
            number,
            currency: currency.into(),
            date: None,
            label: None,
        }
    }

    /// 设置日期
    pub fn with_date(mut self, date: NaiveDate) -> Self {
        self.date = Some(date);
        self
    }

    /// 设置标签
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl fmt::Display for Cost {
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
