//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::collections::HashMap;

use ::serde::{Deserialize, Serialize};

use crate::model::{
    account::{amount::Amount, cost::Cost, price::Price},
    config::meta_value::MetaValue,
};

/// 过账（账户变动）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    /// 账户名称
    pub account: String,
    /// 金额（可选，为空时由 Beancount 自动计算）
    pub amount: Option<Amount>,
    /// 成本（用于证券等）
    pub cost: Option<Cost>,
    /// 标记为 `{}`，用于按已有持仓成本自动匹配
    #[serde(default)]
    pub inferred_cost: bool,
    /// 价格（用于货币转换或市值记录）
    pub price: Option<Price>,
    /// 过账标记（可选）
    pub flag: Option<char>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, MetaValue>,
}

impl Posting {
    /// 创建新过账
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

    /// 设置金额
    pub fn with_amount(mut self, amount: Amount) -> Self {
        self.amount = Some(amount);
        self
    }

    /// 设置成本
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self.inferred_cost = false;
        self
    }

    /// 设置成本为 `{}`（由 Beancount 按持仓自动匹配）
    pub fn with_inferred_cost(mut self) -> Self {
        self.cost = None;
        self.inferred_cost = true;
        self
    }

    /// 设置价格
    pub fn with_price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /// 设置标记
    pub fn with_flag(mut self, flag: char) -> Self {
        self.flag = Some(flag);
        self
    }

    /// 添加元数据
    pub fn with_meta(mut self, key: impl Into<String>, value: MetaValue) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}
