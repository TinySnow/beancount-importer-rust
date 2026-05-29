//! 第三方支付平台 `Provider` 适配器集合。
//!
//! 该模块统一导出常见第三方支付平台账单适配器：
//! - `alipay`：支付宝；
//! - `jd`：京东支付/账单；
//! - `mt`：美团账单；
//! - `wechat`：微信支付。
//!
//! 这些适配器都采用同一设计：只保留平台标识与默认账户参数，
//! 具体字段解析与交易构建逻辑委托给共享现金流转换模块，降低重复实现成本。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     providers::third_party::{
//!         alipay::AlipayProvider,
//!         jd::JdProvider,
//!         mt::MtProvider,
//!         wechat::WechatProvider,
//!     },
//! };
//!
//! let alipay = AlipayProvider;
//! let jd = JdProvider;
//! let mt = MtProvider;
//! let wechat = WechatProvider;
//!
//! assert_eq!(alipay.name(), "alipay");
//! assert_eq!(jd.name(), "jd");
//! assert_eq!(mt.name(), "mt");
//! assert_eq!(wechat.name(), "wechat");
//! ```

pub mod alipay;
pub mod jd;
pub mod mt;
pub mod wechat;

use std::sync::Arc;

use crate::interface::provider::Provider;

pub fn all() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(alipay::AlipayProvider),
        Arc::new(wechat::WechatProvider),
        Arc::new(jd::JdProvider),
        Arc::new(mt::MtProvider),
    ]
}
