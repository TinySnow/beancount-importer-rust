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
    config::{defaults::CommonDefaultsConfig, output::OutputConfig, provider::ProviderConfig},
    rule::Rule,
};

/// 全局配置（由所有供应商共享）。
///
/// 通常对应主配置文件中的顶层字段，作为导入流程的默认行为定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    /// 默认币种（当供应商记录未提供币种时使用）。
    ///
    /// 该字段为运行时缓存字段，由 `default.currency` 归一化填充。
    #[serde(skip, default = "default_currency")]
    pub default_currency: String,

    /// 默认支出借方账户。
    ///
    /// 该字段为运行时缓存字段，由 `default.expense_account` 归一化填充。
    #[serde(skip)]
    pub default_expense_account: Option<String>,

    /// 默认资产账户。
    ///
    /// 该字段为运行时缓存字段，由 `default.asset_account` 归一化填充。
    #[serde(skip)]
    pub default_asset_account: Option<String>,

    /// 默认收入贷方账户。
    ///
    /// 该字段为运行时缓存字段，由 `default.income_account` 归一化填充。
    #[serde(skip)]
    pub default_income_account: Option<String>,

    /// 默认字段分组（推荐新写法）。
    ///
    /// ```yaml
    /// default:
    ///   asset_account: "Assets:Unknown"
    ///   expense_account: "Expenses:Unknown"
    ///   income_account: "Income:Unknown"
    ///   currency: "CNY"
    /// ```
    ///
    #[serde(default, rename = "default")]
    pub defaults: CommonDefaultsConfig,

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
            defaults: CommonDefaultsConfig::default(),
            global_rules: Vec::new(),
            providers: HashMap::new(),
            output: OutputConfig::default(),
        }
    }
}

impl GlobalConfig {
    /// 归一化 `default:` 分组到运行时缓存字段。
    pub fn normalize_default_group(&mut self) {
        self.default_asset_account = self.defaults.asset_account.clone();
        self.default_expense_account = self.defaults.expense_account.clone();
        self.default_income_account = self.defaults.income_account.clone();
        self.default_currency = self
            .defaults
            .currency
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(default_currency);

        for provider in self.providers.values_mut() {
            provider.normalize_default_group();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalConfig;

    #[test]
    fn normalizes_default_group_fields_when_flat_defaults_missing() {
        let yaml = r#"
default:
  asset_account: "Assets:Unknown"
  expense_account: "Expenses:Unknown"
  income_account: "Income:Unknown"
  currency: "USD"
"#;

        let mut config: GlobalConfig =
            serde_yaml::from_str(yaml).expect("global config should deserialize");
        config.normalize_default_group();

        assert_eq!(
            config.default_asset_account.as_deref(),
            Some("Assets:Unknown")
        );
        assert_eq!(
            config.default_expense_account.as_deref(),
            Some("Expenses:Unknown")
        );
        assert_eq!(
            config.default_income_account.as_deref(),
            Some("Income:Unknown")
        );
        assert_eq!(config.default_currency, "USD");
    }

    #[test]
    fn rejects_flat_defaults_fields() {
        let yaml = r#"
default_currency: "HKD"
default:
  currency: "USD"
"#;

        let result = serde_yaml::from_str::<GlobalConfig>(yaml);
        assert!(
            result.is_err(),
            "legacy flat default fields should be rejected"
        );
    }
}
