//! 证券账单 Provider 入口模块。
//!
//! 该模块汇总并导出所有券商适配器实现，目前包括：
//! - [`futu`]：富途证券账单导入；
//! - [`yinhe`]：银河证券账单导入。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     providers::securities::{futu::FutuProvider, yinhe::YinheProvider},
//! };
//!
//! let futu = FutuProvider;
//! let yinhe = YinheProvider;
//!
//! assert_eq!(futu.name(), "futu");
//! assert_eq!(yinhe.name(), "yinhe");
//! ```

/// 富途证券账单 Provider。
pub mod futu;
/// 银河证券账单 Provider。
pub mod yinhe;

use std::sync::Arc;

use crate::interface::provider::Provider;

pub fn all() -> Vec<Arc<dyn Provider>> {
    vec![Arc::new(futu::FutuProvider), Arc::new(yinhe::YinheProvider)]
}
