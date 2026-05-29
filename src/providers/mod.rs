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

use crate::interface::provider::Provider;

pub fn all_providers() -> Vec<Arc<dyn Provider>> {
    let mut providers = banks::all();
    providers.extend(securities::all());
    providers.extend(third_party::all());
    providers
}
