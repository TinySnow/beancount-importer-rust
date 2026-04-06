//! 供应商配置模型。
//!
//! 本模块定义单个数据供应商（银行/券商/平台）的配置结构，并提供：
//! - 供应商配置与全局配置的合并逻辑。
//! - 证券账户字段的结构化配置读取（`securities_accounts`）。
//! - 表格解析与输出配置的供应商级覆盖。
//!
//! # 配置优先级
//! 1. 供应商显式配置
//! 2. 全局默认配置
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::{
//!     defaults::CommonDefaultsConfig,
//!     global::GlobalConfig,
//!     provider::ProviderConfig,
//! };
//!
//! let mut global = GlobalConfig::default();
//! global.defaults = CommonDefaultsConfig {
//!     asset_account: Some("Assets:Global:Cash".to_string()),
//!     currency: Some("USD".to_string()),
//!     ..CommonDefaultsConfig::default()
//! };
//! global.normalize_default_group();
//!
//! let mut provider = ProviderConfig::default();
//! provider.defaults = CommonDefaultsConfig {
//!     asset_account: Some("Assets:Provider:Cash".to_string()),
//!     ..CommonDefaultsConfig::default()
//! };
//! provider.merge_with_global(&global);
//!
//! assert_eq!(
//!     provider.default_asset_account.as_deref(),
//!     Some("Assets:Provider:Cash")
//! );
//! assert_eq!(provider.default_currency.as_deref(), Some("USD"));
//! ```

use log::trace;
use serde::{Deserialize, Serialize};

use crate::model::{
    config::{
        defaults::CommonDefaultsConfig, global::GlobalConfig, output::OutputConfig,
        tabular_options::TabularOptions,
    },
    rule::Rule,
};

/// 证券场景账户配置。
///
/// 建议新配置统一放在此结构中：
///
/// ```yaml
/// securities_accounts:
///   cash_account: "Assets:Broker:Galaxy:Cash"
///   fee_account: "Expenses:Broker:Galaxy:Fee"
///   pnl_account: "Income:Broker:Galaxy:PnL"
///   repo_interest_account: "Income:Broker:Galaxy:RepoInterest"
///   rounding_account: "Expenses:Broker:Galaxy:Rounding"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SecuritiesAccountsConfig {
    /// 券商现金账户。
    #[serde(default)]
    pub cash_account: Option<String>,

    /// 手续费账户。
    #[serde(default)]
    pub fee_account: Option<String>,

    /// 盈亏账户。
    #[serde(default)]
    pub pnl_account: Option<String>,

    /// 逆回购利息账户。
    #[serde(default)]
    pub repo_interest_account: Option<String>,

    /// 舍入差异账户。
    #[serde(default)]
    pub rounding_account: Option<String>,
}

/// 供应商配置。
///
/// 该结构描述一个供应商从解析到写出的完整参数集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// 供应商显示名称。
    pub name: Option<String>,

    /// 字段映射文件路径。
    pub mapping_file: Option<String>,

    /// 默认资产账户（通用）。
    ///
    /// 该字段为运行时缓存字段，由 `default.asset_account` 归一化填充。
    #[serde(skip)]
    pub default_asset_account: Option<String>,

    /// 默认支出账户（通用）。
    ///
    /// 该字段为运行时缓存字段，由 `default.expense_account` 归一化填充。
    #[serde(skip)]
    pub default_expense_account: Option<String>,

    /// 默认收入账户（通用）。
    ///
    /// 该字段为运行时缓存字段，由 `default.income_account` 归一化填充。
    #[serde(skip)]
    pub default_income_account: Option<String>,

    /// 默认币种（通用）。
    ///
    /// 该字段为运行时缓存字段，由 `default.currency` 归一化填充。
    #[serde(skip)]
    pub default_currency: Option<String>,

    /// 默认字段分组（推荐新写法）。
    ///
    /// ```yaml
    /// default:
    ///   asset_account: "Assets:Wallet:WeChat:Balance"
    ///   expense_account: "Expenses:Unknown"
    ///   income_account: "Income:Unknown"
    ///   currency: "CNY"
    /// ```
    ///
    #[serde(default, rename = "default")]
    pub defaults: CommonDefaultsConfig,

    /// 证券账户子结构（推荐新配置使用）。
    #[serde(default)]
    pub securities_accounts: SecuritiesAccountsConfig,

    /// 历史 lot 预加载文件列表（Beancount）。
    ///
    /// 用于跨账期导入时补充历史持仓，减少卖出分录的 lot 二义性。
    #[serde(default)]
    pub inventory_seed_files: Vec<String>,

    /// 表格解析选项（CSV/电子表格共用）。
    #[serde(default)]
    pub tabular_options: TabularOptions,

    /// 供应商规则列表。
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// 输出格式覆盖项。
    #[serde(default)]
    pub output: OutputConfig,

    /// 文件开头需要跳过的非数据行数。
    ///
    /// 适用于导出文件在真实表头前存在额外说明行的情况。
    #[serde(default)]
    pub skip_header_lines: usize,

    /// 数据是否包含表头行。
    #[serde(default = "default_true")]
    pub has_header_row: bool,
}

/// `has_header_row` 字段默认值工厂函数。
fn default_true() -> bool {
    true
}

/// 返回有效（非空白）字符串引用。
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

impl Default for ProviderConfig {
    /// 创建供应商配置默认实例。
    fn default() -> Self {
        Self {
            name: None,
            mapping_file: None,
            default_asset_account: None,
            default_expense_account: None,
            default_income_account: None,
            default_currency: None,
            defaults: CommonDefaultsConfig::default(),
            securities_accounts: SecuritiesAccountsConfig::default(),
            inventory_seed_files: Vec::new(),
            tabular_options: TabularOptions::default(),
            rules: Vec::new(),
            output: OutputConfig::default(),
            skip_header_lines: 0,
            has_header_row: true,
        }
    }
}

impl ProviderConfig {
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
            .map(str::to_owned);
    }

    /// 合并全局配置（供应商配置优先）。
    ///
    /// 仅当供应商字段未设置时，才继承 `global` 的默认值。
    /// 输出配置通过 [`OutputConfig::merge_with`] 执行同样策略。
    ///
    /// # 参数
    /// - `global`：全局默认配置。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::config::{
    ///     global::GlobalConfig,
    ///     provider::ProviderConfig,
    /// };
    ///
    /// let mut global = GlobalConfig::default();
    /// global.default_currency = "HKD".to_string();
    ///
    /// let mut provider = ProviderConfig::default();
    /// provider.merge_with_global(&global);
    ///
    /// assert_eq!(provider.default_currency.as_deref(), Some("HKD"));
    /// ```
    pub fn merge_with_global(&mut self, global: &GlobalConfig) {
        self.normalize_default_group();

        // 通用账户与币种按“供应商优先、全局兜底”合并。
        if self.default_asset_account.is_none() {
            self.default_asset_account = global.default_asset_account.clone();
        }
        if self.default_expense_account.is_none() {
            self.default_expense_account = global.default_expense_account.clone();
        }
        if self.default_income_account.is_none() {
            self.default_income_account = global.default_income_account.clone();
        }
        if self.default_currency.is_none() {
            self.default_currency = Some(global.default_currency.clone());
        }

        self.output.merge_with(&global.output);

        trace!("Merged provider output config: {:?}", self.output);
    }

    /// 获取证券场景有效现金账户。
    pub fn securities_cash_account(&self) -> Option<&str> {
        non_empty(self.securities_accounts.cash_account.as_deref())
    }

    /// 获取证券场景有效手续费账户。
    pub fn securities_fee_account(&self) -> Option<&str> {
        non_empty(self.securities_accounts.fee_account.as_deref())
    }

    /// 获取证券场景有效盈亏账户。
    pub fn securities_pnl_account(&self) -> Option<&str> {
        non_empty(self.securities_accounts.pnl_account.as_deref())
    }

    /// 获取证券场景有效逆回购利息账户。
    pub fn securities_repo_interest_account(&self) -> Option<&str> {
        non_empty(self.securities_accounts.repo_interest_account.as_deref())
    }

    /// 获取证券场景有效舍入差异账户。
    pub fn securities_rounding_account(&self) -> Option<&str> {
        non_empty(self.securities_accounts.rounding_account.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderConfig;

    #[test]
    fn reads_nested_securities_cash_account() {
        let yaml = r#"
securities_accounts:
  cash_account: "Assets:Broker:Alias:Cash"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(
            config.securities_cash_account(),
            Some("Assets:Broker:Alias:Cash")
        );
    }

    #[test]
    fn reads_nested_securities_repo_interest_account() {
        let yaml = r#"
securities_accounts:
  repo_interest_account: "Income:Broker:Alias:RepoInterest"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(
            config.securities_repo_interest_account(),
            Some("Income:Broker:Alias:RepoInterest")
        );
    }

    #[test]
    fn deserializes_inventory_seed_files() {
        let yaml = r#"
inventory_seed_files:
  - "transactions/2025/12/galaxy.bean"
  - "transactions/2025/11/galaxy.bean"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(
            config.inventory_seed_files,
            vec![
                "transactions/2025/12/galaxy.bean".to_string(),
                "transactions/2025/11/galaxy.bean".to_string()
            ]
        );
    }

    #[test]
    fn rejects_legacy_securities_default_fields() {
        let yaml = r#"
default_cash_account: "Assets:Legacy:Cash"
securities_accounts:
  cash_account: "Assets:Nested:Cash"
  fee_account: "Expenses:Nested:Fee"
"#;

        let result = serde_yaml::from_str::<ProviderConfig>(yaml);
        assert!(result.is_err(), "legacy securities flat keys should be rejected");

        let legacy_only_yaml = r#"
default_cash_account: "Assets:Legacy:Cash"
default_fee_account: "Expenses:Legacy:Fee"
default_repo_interest_account: "Income:Legacy:Repo"
"#;
        let legacy_only = serde_yaml::from_str::<ProviderConfig>(legacy_only_yaml);
        assert!(
            legacy_only.is_err(),
            "legacy securities flat keys should be rejected"
        );
    }

    #[test]
    fn rejects_legacy_tabular_option_aliases() {
        let yaml = r#"
csv_options:
  delimiter: ";"
  quote: "'"
  flexible: true
  encoding: "GBK"
has_csv_header: false
"#;

        let result = serde_yaml::from_str::<ProviderConfig>(yaml);
        assert!(
            result.is_err(),
            "legacy tabular aliases should be rejected"
        );
    }

    #[test]
    fn deserializes_tabular_option_new_keys() {
        let yaml = r#"
tabular_options:
  delimiter: ","
  quote: "\""
  flexible: false
  encoding: "UTF-8"
has_header_row: true
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(config.tabular_options.delimiter, ',');
        assert_eq!(config.tabular_options.quote, '"');
        assert!(!config.tabular_options.flexible);
        assert_eq!(config.tabular_options.encoding, "UTF-8");
        assert!(config.has_header_row);
    }

    #[test]
    fn normalizes_default_group_fields_when_flat_defaults_missing() {
        let yaml = r#"
default:
  asset_account: "Assets:Wallet:WeChat:Balance"
  expense_account: "Expenses:Unknown"
  income_account: "Income:Unknown"
  currency: "CNY"
"#;

        let mut config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");
        config.normalize_default_group();

        assert_eq!(
            config.default_asset_account.as_deref(),
            Some("Assets:Wallet:WeChat:Balance")
        );
        assert_eq!(
            config.default_expense_account.as_deref(),
            Some("Expenses:Unknown")
        );
        assert_eq!(
            config.default_income_account.as_deref(),
            Some("Income:Unknown")
        );
        assert_eq!(config.default_currency.as_deref(), Some("CNY"));
    }

    #[test]
    fn rejects_flat_defaults_fields() {
        let yaml = r#"
default_asset_account: "Assets:Flat"
default:
  asset_account: "Assets:Grouped"
"#;

        let result = serde_yaml::from_str::<ProviderConfig>(yaml);
        assert!(
            result.is_err(),
            "legacy flat default fields should be rejected"
        );
    }
}
