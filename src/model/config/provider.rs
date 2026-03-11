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

    /// 供应商级默认舍入差异账户。
    ///
    /// 向后兼容别名：`rounding_account`。
    #[serde(alias = "rounding_account")]
    pub default_rounding_account: Option<String>,

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
            default_expense_account: None,
            default_income_account: None,
            default_currency: None,
            default_fee_account: None,
            default_pnl_account: None,
            default_rounding_account: None,
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
