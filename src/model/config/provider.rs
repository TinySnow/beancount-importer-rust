//! 供应商配置模型。
//!
//! 本模块定义单个数据供应商（银行/券商/平台）的配置结构，并提供：
//! - 供应商配置与全局配置的合并逻辑。
//! - 证券账户字段的新旧配置兼容（嵌套字段优先，历史字段兜底）。
//! - 表格解析与输出配置的供应商级覆盖。
//!
//! # 配置优先级
//! 1. 供应商显式配置
//! 2. 全局默认配置
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::{
//!     global::GlobalConfig,
//!     provider::ProviderConfig,
//! };
//!
//! let mut global = GlobalConfig::default();
//! global.default_currency = "USD".to_string();
//! global.default_asset_account = Some("Assets:Global:Cash".to_string());
//!
//! let mut provider = ProviderConfig::default();
//! provider.default_asset_account = Some("Assets:Provider:Cash".to_string());
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
    config::{global::GlobalConfig, output::OutputConfig, tabular_options::TabularOptions},
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
pub struct SecuritiesAccountsConfig {
    /// 券商现金账户。
    #[serde(default, alias = "default_cash_account")]
    pub cash_account: Option<String>,

    /// 手续费账户。
    #[serde(default, alias = "default_fee_account")]
    pub fee_account: Option<String>,

    /// 盈亏账户。
    #[serde(default, alias = "default_pnl_account")]
    pub pnl_account: Option<String>,

    /// 逆回购利息账户。
    #[serde(default, alias = "default_repo_interest_account")]
    pub repo_interest_account: Option<String>,

    /// 舍入差异账户。
    #[serde(default, alias = "default_rounding_account")]
    pub rounding_account: Option<String>,
}

/// 供应商配置。
///
/// 该结构描述一个供应商从解析到写出的完整参数集合。
/// 在反序列化时同时兼容新字段与旧字段命名。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 供应商显示名称。
    pub name: Option<String>,

    /// 字段映射文件路径。
    pub mapping_file: Option<String>,

    /// 默认资产账户（通用）。
    pub default_asset_account: Option<String>,

    /// 默认支出账户（通用）。
    pub default_expense_account: Option<String>,

    /// 默认收入账户（通用）。
    pub default_income_account: Option<String>,

    /// 默认币种（通用）。
    pub default_currency: Option<String>,

    /// 证券账户子结构（推荐新配置使用）。
    #[serde(default)]
    pub securities_accounts: SecuritiesAccountsConfig,

    /// 兼容字段：默认券商现金账户（证券场景）。
    ///
    /// 向后兼容别名：`cash_account`。
    #[serde(alias = "cash_account")]
    pub default_cash_account: Option<String>,

    /// 兼容字段：默认手续费账户。
    ///
    /// 向后兼容别名：`fee_account`。
    #[serde(alias = "fee_account")]
    pub default_fee_account: Option<String>,

    /// 兼容字段：默认盈亏账户。
    ///
    /// 向后兼容别名：`pnl_account`。
    #[serde(alias = "pnl_account")]
    pub default_pnl_account: Option<String>,

    /// 兼容字段：默认逆回购利息账户。
    ///
    /// 向后兼容别名：`repo_interest_account`。
    #[serde(alias = "repo_interest_account")]
    pub default_repo_interest_account: Option<String>,

    /// 兼容字段：默认舍入差异账户。
    ///
    /// 向后兼容别名：`rounding_account`。
    #[serde(alias = "rounding_account")]
    pub default_rounding_account: Option<String>,

    /// 历史 lot 预加载文件列表（Beancount）。
    ///
    /// 用于跨账期导入时补充历史持仓，减少卖出分录的 lot 二义性。
    /// 向后兼容别名：`lot_seed_files`、`history_beancount_files`。
    #[serde(default, alias = "lot_seed_files", alias = "history_beancount_files")]
    pub inventory_seed_files: Vec<String>,

    /// 表格解析选项（CSV/电子表格共用）。
    #[serde(default, alias = "csv_options")]
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
    #[serde(default = "default_true", alias = "has_csv_header")]
    pub has_header_row: bool,
}

/// `has_header_row` 字段默认值工厂函数。
fn default_true() -> bool {
    true
}

/// 返回首个有效（非空白）字符串引用。
///
/// 优先使用 `primary`，当其为空或仅含空白时回退到 `fallback`。
/// 该函数用于新旧配置字段并存时的统一取值逻辑。
fn first_non_empty<'a>(primary: Option<&'a str>, fallback: Option<&'a str>) -> Option<&'a str> {
    primary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        // 仅当 primary 缺失或为空时，才使用 fallback。
        .or_else(|| fallback.map(str::trim).filter(|value| !value.is_empty()))
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
            securities_accounts: SecuritiesAccountsConfig::default(),
            default_cash_account: None,
            default_fee_account: None,
            default_pnl_account: None,
            default_repo_interest_account: None,
            default_rounding_account: None,
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
    ///
    /// 优先读取 `securities_accounts.cash_account`，再回退到
    /// 历史字段 `default_cash_account`。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::config::provider::ProviderConfig;
    ///
    /// let mut config = ProviderConfig::default();
    /// config.default_cash_account = Some("Assets:Legacy:Cash".to_string());
    /// assert_eq!(config.securities_cash_account(), Some("Assets:Legacy:Cash"));
    ///
    /// config.securities_accounts.cash_account = Some("Assets:Nested:Cash".to_string());
    /// assert_eq!(config.securities_cash_account(), Some("Assets:Nested:Cash"));
    /// ```
    pub fn securities_cash_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.cash_account.as_deref(),
            self.default_cash_account.as_deref(),
        )
    }

    /// 获取证券场景有效手续费账户。
    ///
    /// 优先读取嵌套字段 `securities_accounts.fee_account`，否则回退到
    /// 历史字段 `default_fee_account`。
    pub fn securities_fee_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.fee_account.as_deref(),
            self.default_fee_account.as_deref(),
        )
    }

    /// 获取证券场景有效盈亏账户。
    ///
    /// 优先读取嵌套字段 `securities_accounts.pnl_account`，否则回退到
    /// 历史字段 `default_pnl_account`。
    pub fn securities_pnl_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.pnl_account.as_deref(),
            self.default_pnl_account.as_deref(),
        )
    }

    /// 获取证券场景有效逆回购利息账户。
    ///
    /// 优先读取嵌套字段 `securities_accounts.repo_interest_account`，
    /// 否则回退到历史字段 `default_repo_interest_account`。
    pub fn securities_repo_interest_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.repo_interest_account.as_deref(),
            self.default_repo_interest_account.as_deref(),
        )
    }

    /// 获取证券场景有效舍入差异账户。
    ///
    /// 优先读取嵌套字段 `securities_accounts.rounding_account`，否则回退到
    /// 历史字段 `default_rounding_account`。
    pub fn securities_rounding_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.rounding_account.as_deref(),
            self.default_rounding_account.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderConfig;

    #[test]
    fn deserializes_cash_account_alias() {
        let yaml = r#"
cash_account: "Assets:Broker:Alias:Cash"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(
            config.default_cash_account.as_deref(),
            Some("Assets:Broker:Alias:Cash")
        );
        assert_eq!(
            config.securities_cash_account(),
            Some("Assets:Broker:Alias:Cash")
        );
    }

    #[test]
    fn deserializes_repo_interest_account_alias() {
        let yaml = r#"
repo_interest_account: "Income:Broker:Alias:RepoInterest"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(
            config.default_repo_interest_account.as_deref(),
            Some("Income:Broker:Alias:RepoInterest")
        );
        assert_eq!(
            config.securities_repo_interest_account(),
            Some("Income:Broker:Alias:RepoInterest")
        );
    }

    #[test]
    fn deserializes_inventory_seed_files_alias() {
        let yaml = r#"
lot_seed_files:
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
    fn prefers_nested_securities_accounts_over_legacy_fields() {
        let yaml = r#"
default_cash_account: "Assets:Legacy:Cash"
securities_accounts:
  cash_account: "Assets:Nested:Cash"
  fee_account: "Expenses:Nested:Fee"
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(config.securities_cash_account(), Some("Assets:Nested:Cash"));
        assert_eq!(config.securities_fee_account(), Some("Expenses:Nested:Fee"));
    }

    #[test]
    fn deserializes_legacy_tabular_option_aliases() {
        let yaml = r#"
csv_options:
  delimiter: ";"
  quote: "'"
  flexible: true
  encoding: "GBK"
has_csv_header: false
"#;

        let config: ProviderConfig =
            serde_yaml::from_str(yaml).expect("provider config should deserialize");

        assert_eq!(config.tabular_options.delimiter, ';');
        assert_eq!(config.tabular_options.quote, '\'');
        assert!(config.tabular_options.flexible);
        assert_eq!(config.tabular_options.encoding, "GBK");
        assert!(!config.has_header_row);
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
}
