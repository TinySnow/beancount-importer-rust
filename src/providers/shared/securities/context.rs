//! 证券记录标准上下文模型。
//!
//! 将 `RawRecord` 中证券转换所需字段集中收敛为单一结构，
//! 以降低下游构建函数的参数复杂度并统一基础校验。

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::{
    error::{ImporterError, ImporterResult},
    model::data::raw_record::RawRecord,
};

/// 证券记录标准上下文。
///
/// 该结构保留证券转换过程中可能使用到的所有字段：
/// - 必填字段在构造阶段校验（如 `date`）；
/// - 其余字段按 `Option` 传递，由具体交易类型自行决定是否必需。
#[derive(Debug, Clone)]
pub(super) struct SecurityRecordContext {
    /// 交易日期（必填）。
    pub(super) date: NaiveDate,
    /// 现金金额，通常为成交总额或划转金额。
    pub(super) amount: Option<Decimal>,
    /// 现金币种（已归一为标准代码，如 `CNY`、`USD`）。
    pub(super) cash_currency: String,
    /// 交易对手。
    pub(super) payee: Option<String>,
    /// 交易摘要。
    pub(super) narration: Option<String>,
    /// Provider 原始交易类型文本。
    pub(super) transaction_type: Option<String>,
    /// 订单号或参考号。
    pub(super) reference: Option<String>,
    /// 证券代码。
    pub(super) symbol: Option<String>,
    /// 证券名称。
    pub(super) security_name: Option<String>,
    /// 成交数量。
    pub(super) quantity: Option<Decimal>,
    /// 成交单价。
    pub(super) unit_price: Option<Decimal>,
    /// 手续费。
    pub(super) fee: Option<Decimal>,
    /// 税费。
    pub(super) tax: Option<Decimal>,
    /// 其余未标准化字段。
    pub(super) extra: HashMap<String, String>,
}

impl SecurityRecordContext {
    /// 从原始记录构造证券上下文，并完成基础字段校验。
    ///
    /// 当前仅强制校验 `date`，其余字段由后续“现金划转”或“证券交易”
    /// 路径按需校验，以兼容不同 Provider 的字段完整度差异。
    pub(super) fn from_record(record: RawRecord, cash_currency: String) -> ImporterResult<Self> {
        let RawRecord {
            date,
            amount,
            payee,
            narration,
            transaction_type,
            reference,
            symbol,
            security_name,
            quantity,
            unit_price,
            fee,
            tax,
            extra,
            ..
        } = record;

        let date =
            date.ok_or_else(|| ImporterError::Conversion("Missing trade date".to_string()))?;

        Ok(Self {
            date,
            amount,
            cash_currency,
            payee,
            narration,
            transaction_type,
            reference,
            symbol,
            security_name,
            quantity,
            unit_price,
            fee,
            tax,
            extra,
        })
    }
}
