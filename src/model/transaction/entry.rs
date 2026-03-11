//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! Beancount 交易模型定义

use crate::model::account::posting::Posting;
use crate::model::config::meta_value::MetaValue;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Beancount 交易
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易日期
    pub date: NaiveDate,
    /// 交易标记（`*` 表示已确认，`!` 表示待确认）
    #[serde(default = "default_flag")]
    pub flag: char,
    /// 交易对手（可选）
    pub payee: Option<String>,
    /// 交易描述
    pub narration: String,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 链接列表
    #[serde(default)]
    pub links: Vec<String>,
    /// 过账列表
    pub postings: Vec<Posting>,
    /// 元数据（可扩展字段）
    #[serde(default)]
    pub metadata: HashMap<String, MetaValue>,
}

fn default_flag() -> char {
    '*'
}

impl Transaction {
    /// 创建新交易
    pub fn new(date: NaiveDate, narration: impl Into<String>) -> Self {
        Self {
            date,
            flag: '*',
            payee: None,
            narration: narration.into(),
            tags: Vec::new(),
            links: Vec::new(),
            postings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// 设置交易对手
    pub fn with_payee(mut self, payee: impl Into<String>) -> Self {
        self.payee = Some(payee.into());
        self
    }

    /// 设置标记
    pub fn with_flag(mut self, flag: char) -> Self {
        self.flag = flag;
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// 添加链接
    pub fn with_link(mut self, link: impl Into<String>) -> Self {
        self.links.push(link.into());
        self
    }

    /// 添加过账
    pub fn with_posting(mut self, posting: Posting) -> Self {
        self.postings.push(posting);
        self
    }

    /// 添加元数据
    pub fn with_meta(mut self, key: impl Into<String>, value: MetaValue) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// 验证交易是否平衡（简化版本，不考虑不同货币）
    pub fn is_balanced(&self) -> bool {
        // 按货币分组计算
        let mut balances: HashMap<&str, rust_decimal::Decimal> = HashMap::new();

        for posting in &self.postings {
            if let Some(ref amount) = posting.amount {
                *balances.entry(&amount.currency).or_default() += amount.number;
            }
        }

        // 允许一个过账金额为空（自动平衡）
        let empty_amount_count = self.postings.iter().filter(|p| p.amount.is_none()).count();

        if empty_amount_count > 1 {
            return false;
        }

        if empty_amount_count == 1 {
            return true; // Beancount 会自动计算
        }

        balances.values().all(|b| b.is_zero())
    }
}
