//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
//! 供应商注册表。

use std::{collections::HashMap, sync::Arc};

use crate::{interface::provider::Provider, providers::GLOBAL_REGISTRY};

/// 运行时供应商注册表。
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// 全局静态注册表（内置供应商）。
    pub fn global() -> &'static ProviderRegistry {
        &GLOBAL_REGISTRY
    }

    /// 注册一个供应商。
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        self.providers
            .insert(provider.name().to_lowercase(), provider);
    }

    /// 按名称获取供应商。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(&name.to_lowercase()).cloned()
    }

    /// 以稳定排序列出所有供应商名称。
    pub fn list_providers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers.keys().map(|value| value.as_str()).collect();
        names.sort_unstable();
        names
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
