//! Provider 层共享逻辑模块。

pub(crate) mod cashflow;
pub(crate) mod securities;
pub(crate) mod transaction_enricher;

pub(crate) use cashflow::{CashflowTransformOptions, transform_cashflow_record};
pub(crate) use securities::{SecurityTransformOptions, transform_security_record};
pub(crate) use transaction_enricher::{append_extra_metadata, append_order_id, apply_match_result};
