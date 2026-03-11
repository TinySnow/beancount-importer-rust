//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
