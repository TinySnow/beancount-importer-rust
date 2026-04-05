//! 跨 Provider 的共享转换能力。
//!
//! 本模块聚合两类通用转换流程：
//! - 现金流类转换（银行卡、钱包、第三方支付）；
//! - 证券类转换（现货交易、逆回购、银证转账）。
//!
//! 同时导出交易补充工具函数，用于统一写入 `source`、`orderId` 和扩展元数据。

pub(crate) mod cashflow;
pub(crate) mod securities;
pub(crate) mod transaction_enricher;

pub(crate) use cashflow::{CashflowTransformOptions, transform_cashflow_record};
pub(crate) use securities::{SecurityTransformOptions, transform_security_record};
pub(crate) use transaction_enricher::{append_extra_metadata, append_order_id, apply_match_result};
