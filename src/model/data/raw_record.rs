//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! 原始账单记录模型。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 从源账单解析出的标准化中间记录。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawRecord {
    pub date: Option<NaiveDate>,
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
    pub payee: Option<String>,
    pub narration: Option<String>,
    pub transaction_type: Option<String>,
    pub status: Option<String>,
    pub reference: Option<String>,

    // 证券相关字段。
    pub symbol: Option<String>,
    pub security_name: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub fee: Option<Decimal>,
    pub tax: Option<Decimal>,

    /// 供应商专属扩展字段。
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

impl RawRecord {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从标准字段或扩展字段中获取值。
    pub fn get(&self, field: &str) -> Option<&str> {
        match field {
            "payee" => self.payee.as_deref(),
            "narration" => self.narration.as_deref(),
            "transaction_type" => self.transaction_type.as_deref(),
            "status" => self.status.as_deref(),
            "reference" => self.reference.as_deref(),
            "symbol" => self.symbol.as_deref(),
            "security_name" => self.security_name.as_deref(),
            "currency" => self.currency.as_deref(),
            "peer" => self.extra.get("peer").map(String::as_str),
            "peerAccount" => self.extra.get("peerAccount").map(String::as_str),
            _ => self.extra.get(field).map(String::as_str),
        }
    }

    pub fn set_extra(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.extra.insert(key.into(), value.into());
    }

    pub fn is_security_transaction(&self) -> bool {
        self.symbol.is_some() && self.quantity.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::RawRecord;

    #[test]
    fn peer_fields_are_not_aliased_to_counterparty() {
        let mut record = RawRecord::new();
        record.set_extra("peer", "A");
        record.set_extra("peerAccount", "B");

        assert_eq!(record.get("peer"), Some("A"));
        assert_eq!(record.get("peerAccount"), Some("B"));
        assert_eq!(record.get("counterparty"), None);
        assert_eq!(record.get("counterpartyAccount"), None);
    }
}
