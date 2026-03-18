//! 模块说明：Provider 模块统一导出入口。
//!
//! 文件路径：src/providers/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：banks、securities、third_party。

pub mod banks;
pub mod securities;
pub(crate) mod shared;
pub mod third_party;

use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::model::registry::provider_registry::ProviderRegistry;

/// 全局供应商注册表。
pub static GLOBAL_REGISTRY: Lazy<ProviderRegistry> = Lazy::new(|| {
    let mut registry = ProviderRegistry::new();

    registry.register(Arc::new(third_party::alipay::AlipayProvider));
    registry.register(Arc::new(third_party::wechat::WechatProvider));
    registry.register(Arc::new(third_party::jd::JdProvider));
    registry.register(Arc::new(third_party::mt::MtProvider));
    registry.register(Arc::new(banks::icbc::IcbcProvider));
    registry.register(Arc::new(banks::ccb::CcbProvider));
    registry.register(Arc::new(securities::futu::FutuProvider));
    registry.register(Arc::new(securities::yinhe::YinheProvider));

    registry
});
