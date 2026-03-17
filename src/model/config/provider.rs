//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use log::trace;
use serde::{Deserialize, Serialize};

use crate::model::{
    config::{csv_options::CsvOptions, global::GlobalConfig, output::OutputConfig},
    rule::Rule,
};

/// 供应商专属配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 供应商显示名称。
    pub name: Option<String>,

    /// 字段映射文件路径。
    pub mapping_file: Option<String>,

    /// 供应商级默认资产账户。
    pub default_asset_account: Option<String>,

    /// 供应商级默认券商现金账户（证券场景）。
    ///
    /// 向后兼容别名：`cash_account`。
    #[serde(alias = "cash_account")]
    pub default_cash_account: Option<String>,

    /// 供应商级默认支出账户。
    pub default_expense_account: Option<String>,

    /// 供应商级默认收入账户。
    pub default_income_account: Option<String>,

    /// 供应商级默认币种。
    pub default_currency: Option<String>,

    /// 供应商级默认手续费账户。
    ///
    /// 向后兼容别名：`fee_account`。
    #[serde(alias = "fee_account")]
    pub default_fee_account: Option<String>,

    /// 供应商级默认盈亏账户。
    ///
    /// 向后兼容别名：`pnl_account`。
    #[serde(alias = "pnl_account")]
    pub default_pnl_account: Option<String>,

    /// 供应商级默认逆回购利息账户。
    ///
    /// 向后兼容别名：`repo_interest_account`。
    #[serde(alias = "repo_interest_account")]
    pub default_repo_interest_account: Option<String>,

    /// 供应商级默认舍入差异账户。
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

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: None,
            mapping_file: None,
            default_asset_account: None,
            default_cash_account: None,
            default_expense_account: None,
            default_income_account: None,
            default_currency: None,
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
    /// 与全局配置合并（供应商配置优先）。
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
}
