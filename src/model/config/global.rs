//! 全局配置模型。
//!
//! 本模块定义所有供应商共享的默认配置项，包含默认账户、默认币种、
//! 全局规则列表、嵌入式供应商配置以及输出选项。
//!
//! # 设计约束
//! - 供应商配置优先于全局配置。
//! - 全局配置用于填充供应商未显式设置的字段。
//! - 反序列化缺省时，`default_currency` 默认值为 `CNY`。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::global::GlobalConfig;
//!
//! let config = GlobalConfig::default();
//! assert_eq!(config.default_currency, "CNY");
//! assert!(config.global_rules.is_empty());
//! assert!(config.providers.is_empty());
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    config::{output::OutputConfig, provider::ProviderConfig},
    rule::Rule,
};

/// 全局配置（由所有供应商共享）。
///
/// 通常对应主配置文件中的顶层字段，作为导入流程的默认行为定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 默认币种（当供应商记录未提供币种时使用）。
    #[serde(default = "default_currency")]
    pub default_currency: String,

    /// 默认支出借方账户。
    pub default_expense_account: Option<String>,

    /// 默认资产账户。
    pub default_asset_account: Option<String>,

    /// 默认收入贷方账户。
    pub default_income_account: Option<String>,

    /// 全局规则（优先级低于供应商规则）。
    #[serde(default)]
    pub global_rules: Vec<Rule>,

    /// 汇总在同一全局文件中的供应商配置。
    ///
    /// 键通常是供应商标识（如 `cmb`、`icbc`）。
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// 输出格式默认配置。
    #[serde(default)]
    pub output: OutputConfig,
}

/// `default_currency` 字段的默认值工厂函数。
fn default_currency() -> String {
    "CNY".to_string()
}

impl Default for GlobalConfig {
    /// 创建全局配置的默认实例。
    ///
    /// 该默认值会被用于：
    /// - 手动初始化配置对象。
    /// - 配置文件字段缺失时的序列化默认填充。
    fn default() -> Self {
        Self {
            default_currency: default_currency(),
            default_expense_account: None,
            default_asset_account: None,
            default_income_account: None,
            global_rules: Vec::new(),
            providers: HashMap::new(),
            output: OutputConfig::default(),
        }
    }
}
