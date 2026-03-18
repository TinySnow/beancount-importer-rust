//! 模块说明：Provider 共享逻辑模块。
//!
//! 文件路径：src/providers/shared/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

pub(crate) mod cashflow;
pub(crate) mod securities;
pub(crate) mod transaction_enricher;

pub(crate) use cashflow::{CashflowTransformOptions, transform_cashflow_record};
pub(crate) use securities::{SecurityTransformOptions, transform_security_record};
pub(crate) use transaction_enricher::{append_extra_metadata, append_order_id, apply_match_result};
