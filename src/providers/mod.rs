//! Provider 适配器集合。
//!
//! 该模块聚合所有内置供应商实现：
//! - [`banks`]：银行账单适配器；
//! - [`securities`]：证券账单适配器；
//! - [`third_party`]：第三方支付适配器；
//! - [`shared`]：跨供应商转换逻辑（内部使用）。

pub mod banks;
pub mod securities;
pub(crate) mod shared;
pub mod third_party;

use std::sync::Arc;

use crate::interface::provider::Provider;

/// 收集所有内置供应商实例。
///
/// 用于初始化全局注册表（`GLOBAL_REGISTRY`）。
pub fn all_providers() -> Vec<Arc<dyn Provider>> {
    let mut providers = banks::all();
    providers.extend(securities::all());
    providers.extend(third_party::all());
    providers
}
