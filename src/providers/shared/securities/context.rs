use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::{
    error::{ImporterError, ImporterResult},
    model::data::raw_record::RawRecord,
};

/// 证券记录标准上下文。
///
/// 用于把 `RawRecord` 中证券转换所需字段一次性归一化，
/// 降低后续“交易构建函数”的参数传递链路。
#[derive(Debug, Clone)]
pub(super) struct SecurityRecordContext {
    pub(super) date: NaiveDate,
    pub(super) amount: Option<Decimal>,
    pub(super) cash_currency: String,
    pub(super) payee: Option<String>,
    pub(super) narration: Option<String>,
    pub(super) transaction_type: Option<String>,
    pub(super) reference: Option<String>,
    pub(super) symbol: Option<String>,
    pub(super) security_name: Option<String>,
    pub(super) quantity: Option<Decimal>,
    pub(super) unit_price: Option<Decimal>,
    pub(super) fee: Option<Decimal>,
    pub(super) tax: Option<Decimal>,
    pub(super) extra: HashMap<String, String>,
}

impl SecurityRecordContext {
    /// 从原始记录构造证券上下文，并完成基础字段校验。
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
