//! 模块说明：配置模型定义与序列化反序列化规则。
//!
//! 文件路径：src/model/config/provider.rs。
//! 该文件聚焦 Provider 接口约定。
//! 关键符号：SecuritiesAccountsConfig、ProviderConfig、default_true、first_non_empty。

use log::trace;
use serde::{Deserialize, Serialize};

use crate::model::{
    config::{csv_options::CsvOptions, global::GlobalConfig, output::OutputConfig},
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

    /// CSV 解析选项。
    #[serde(default)]
    pub csv_options: CsvOptions,

    /// 供应商规则列表。
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// 输出格式覆盖项。
    #[serde(default)]
    pub output: OutputConfig,

    /// 文件开头需要跳过的非数据行数。
    #[serde(default)]
    pub skip_header_lines: usize,

    /// 数据是否包含 CSV 表头行。
    #[serde(default = "default_true")]
    pub has_csv_header: bool,
}

fn default_true() -> bool {
    true
}

fn first_non_empty<'a>(primary: Option<&'a str>, fallback: Option<&'a str>) -> Option<&'a str> {
    primary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| fallback.map(str::trim).filter(|value| !value.is_empty()))
}

impl Default for ProviderConfig {
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
            csv_options: CsvOptions::default(),
            rules: Vec::new(),
            output: OutputConfig::default(),
            skip_header_lines: 0,
            has_csv_header: true,
        }
    }
}

impl ProviderConfig {
    /// 合并全局配置（供应商配置优先）。
    pub fn merge_with_global(&mut self, global: &GlobalConfig) {
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
        first_non_empty(
            self.securities_accounts.cash_account.as_deref(),
            self.default_cash_account.as_deref(),
        )
    }

    /// 获取证券场景有效手续费账户。
    pub fn securities_fee_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.fee_account.as_deref(),
            self.default_fee_account.as_deref(),
        )
    }

    /// 获取证券场景有效盈亏账户。
    pub fn securities_pnl_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.pnl_account.as_deref(),
            self.default_pnl_account.as_deref(),
        )
    }

    /// 获取证券场景有效逆回购利息账户。
    pub fn securities_repo_interest_account(&self) -> Option<&str> {
        first_non_empty(
            self.securities_accounts.repo_interest_account.as_deref(),
            self.default_repo_interest_account.as_deref(),
        )
    }

    /// 获取证券场景有效舍入差异账户。
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
}
