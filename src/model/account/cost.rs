//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
