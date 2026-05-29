//! 银行账单 `Provider` 集合。
//!
//! 本模块集中导出银行类对账单适配器。各子模块均实现
//! [`Provider`](crate::interface::provider::Provider) trait，
//! 并复用共享现金流转换逻辑完成 `RawRecord -> Transaction` 映射。
//!
//! # 包含的适配器
//! - [`ccb`]：中国建设银行（CCB）
//! - [`icbc`]：中国工商银行（ICBC）
//! - [`dzccb`]：达州银行（DZCCB）
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     providers::banks::{ccb::CcbProvider, dzccb::DzccbProvider, icbc::IcbcProvider},
//! };
//!
//! let providers: Vec<Box<dyn Provider>> = vec![
//!     Box::new(CcbProvider),
//!     Box::new(IcbcProvider),
//!     Box::new(DzccbProvider),
//! ];
//!
//! let names: Vec<&str> = providers.iter().map(|provider| provider.name()).collect();
//! assert_eq!(names, vec!["ccb", "icbc", "dzccb"]);
//! ```

pub mod ccb;
pub mod dzccb;
pub mod icbc;

use std::sync::Arc;

use crate::interface::provider::Provider;

pub fn all() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(ccb::CcbProvider),
        Arc::new(icbc::IcbcProvider),
        Arc::new(dzccb::DzccbProvider),
    ]
}
