//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    config::{output::OutputConfig, provider::ProviderConfig},
    rule::Rule,
};

/// 全局配置（由所有供应商共享）。
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
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// 输出格式默认配置。
    #[serde(default)]
    pub output: OutputConfig,
}

fn default_currency() -> String {
    "CNY".to_string()
}

impl Default for GlobalConfig {
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
