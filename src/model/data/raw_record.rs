//! 原始交易记录模型。
//!
//! [`RawRecord`] 表示从银行、券商或支付平台账单解析得到的一条中间记录。
//! 该类型尽量保留上游原始语义，并通过 `extra` 字段承载供应商特有字段，
//! 以便后续映射规则统一读取。
//!
//! # 主要职责
//! - 保存跨供应商通用字段（日期、金额、币种、摘要等）
//! - 保存证券场景字段（标的、数量、单价、费用、税费等）
//! - 通过 [`RawRecord::get`] 提供统一的“按字段名取值”能力
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::data::raw_record::RawRecord;
//! use chrono::NaiveDate;
//! use rust_decimal::Decimal;
//!
//! let mut record = RawRecord::new();
//! record.date = Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
//! record.amount = Some(Decimal::from_str_exact("100.00").unwrap());
//! record.currency = Some("CNY".to_string());
//! record.transaction_type = Some("deposit".to_string());
//! record.set_extra("type", "income");
//!
//! assert_eq!(record.get("currency"), Some("CNY"));
//! assert_eq!(record.get("type"), Some("income"));
//! assert!(!record.is_security_transaction());
//! ```

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 从源账单解析出的标准化中间记录。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawRecord {
    /// 交易日期。
    pub date: Option<NaiveDate>,
    /// 交易金额（正负方向由上游解析规则决定）。
    pub amount: Option<Decimal>,
    /// 交易币种或商品代码（例如 `CNY`、`USD`）。
    pub currency: Option<String>,
    /// 收款方/商户/对手方名称。
    pub payee: Option<String>,
    /// 交易摘要或备注。
    pub narration: Option<String>,
    /// 标准化交易类型（例如 `income`、`expense`、`transfer`）。
    pub transaction_type: Option<String>,
    /// 上游记录状态（例如成功、撤销、待入账）。
    pub status: Option<String>,
    /// 交易流水号、订单号或外部引用。
    pub reference: Option<String>,

    /// 证券代码（例如 `AAPL`、`510300`）。
    pub symbol: Option<String>,
    /// 证券名称。
    pub security_name: Option<String>,
    /// 成交数量。
    pub quantity: Option<Decimal>,
    /// 成交单价。
    pub unit_price: Option<Decimal>,
    /// 手续费等交易费用。
    pub fee: Option<Decimal>,
    /// 税费金额。
    pub tax: Option<Decimal>,

    /// 供应商专属扩展字段。
    ///
    /// 使用 `#[serde(flatten)]` 后，序列化/反序列化时这些键值会直接并入对象顶层，
    /// 方便接收结构不固定的上游数据。
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

impl RawRecord {
    /// 创建一条空的原始记录。
    ///
    /// # 返回值
    /// 返回所有标准字段均为 `None`、`extra` 为空映射的 [`RawRecord`]。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::data::raw_record::RawRecord;
    ///
    /// let record = RawRecord::new();
    /// assert!(record.amount.is_none());
    /// assert!(record.extra.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 按字段名读取字符串值。
    ///
    /// 读取顺序如下：
    /// 1. 优先匹配内置标准字段（如 `payee`、`currency`）。
    /// 2. 对于 `type`，优先读取 `extra.type`，若不存在再回退到 `transaction_type`。
    /// 3. 未命中标准字段时，回退读取 `extra[field]`。
    ///
    /// # 参数
    /// - `field`：要读取的字段名
    ///
    /// # 返回值
    /// 返回字段对应的字符串切片；若字段不存在则返回 `None`。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::data::raw_record::RawRecord;
    ///
    /// let mut record = RawRecord::new();
    /// record.transaction_type = Some("expense".to_string());
    /// record.set_extra("type", "custom-expense");
    /// record.set_extra("peer", "Alice");
    ///
    /// assert_eq!(record.get("type"), Some("custom-expense"));
    /// assert_eq!(record.get("transaction_type"), Some("expense"));
    /// assert_eq!(record.get("peer"), Some("Alice"));
    /// assert_eq!(record.get("unknown"), None);
    /// ```
    pub fn get(&self, field: &str) -> Option<&str> {
        match field {
            "payee" => self.payee.as_deref(),
            "narration" => self.narration.as_deref(),
            "transaction_type" => self.transaction_type.as_deref(),
            // 兼容规则里常用的 `type` 字段：
            // 优先使用 extra.type；若未显式映射，则回退到标准 transaction_type。
            "type" => self
                .extra
                .get("type")
                .map(String::as_str)
                .or(self.transaction_type.as_deref()),
            "status" => self.status.as_deref(),
            "reference" => self.reference.as_deref(),
            "symbol" => self.symbol.as_deref(),
            "security_name" => self.security_name.as_deref(),
            "currency" => self.currency.as_deref(),
            // 这里保留 `peer` / `peerAccount` 的直读行为，
            // 避免与旧规则中的 `counterparty*` 语义混淆。
            "peer" => self.extra.get("peer").map(String::as_str),
            "peerAccount" => self.extra.get("peerAccount").map(String::as_str),
            _ => self.extra.get(field).map(String::as_str),
        }
    }

    /// 设置供应商扩展字段。
    ///
    /// 若 `key` 已存在，则使用新值覆盖旧值。
    ///
    /// # 参数
    /// - `key`：扩展字段名
    /// - `value`：字段值
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::data::raw_record::RawRecord;
    ///
    /// let mut record = RawRecord::new();
    /// record.set_extra("channel", "bank");
    /// record.set_extra("channel", "broker");
    ///
    /// assert_eq!(record.get("channel"), Some("broker"));
    /// ```
    pub fn set_extra(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.extra.insert(key.into(), value.into());
    }

    /// 判断记录是否为证券交易。
    ///
    /// 当前规则要求同时具备 `symbol` 与 `quantity` 才视为证券交易。
    /// 该约束可以避免把仅有证券名称但无成交数量的普通消费误判为证券流水。
    ///
    /// # 返回值
    /// - `true`：`symbol` 与 `quantity` 均存在
    /// - `false`：否则
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::data::raw_record::RawRecord;
    /// use rust_decimal::Decimal;
    ///
    /// let mut record = RawRecord::new();
    /// assert!(!record.is_security_transaction());
    ///
    /// record.symbol = Some("AAPL".to_string());
    /// record.quantity = Some(Decimal::from_str_exact("10").unwrap());
    /// assert!(record.is_security_transaction());
    /// ```
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
