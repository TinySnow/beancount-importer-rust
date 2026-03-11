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

/// 输出格式配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// 交易写出时使用的日期格式。
    #[serde(default = "default_date_format")]
    pub date_format: String,

    /// 金额格式化时使用的小数位数。
    #[serde(default = "default_decimal_places")]
    pub decimal_places: u32,

    /// 可选账户前缀。
    pub account_prefix: Option<String>,

    /// 是否为当前输出中的所有账户写出 `open` 指令。
    #[serde(default)]
    pub emit_open_directives: bool,

    /// 可选 `open` 日期，格式为 `%Y-%m-%d`。
    /// 若未配置且启用 `emit_open_directives`，则使用最早交易日期。
    pub open_date: Option<String>,
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

fn default_decimal_places() -> u32 {
    2
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            date_format: default_date_format(),
            decimal_places: default_decimal_places(),
            account_prefix: None,
            emit_open_directives: false,
            open_date: None,
        }
    }
}

impl OutputConfig {
    /// 与另一份输出配置合并（当前配置优先）。
    pub fn merge_with(&mut self, other: &OutputConfig) {
        trace!("Merging output config with global defaults");

        if self.date_format == default_date_format() {
            self.date_format = other.date_format.clone();
        }
        if self.decimal_places == default_decimal_places() {
            self.decimal_places = other.decimal_places;
        }
        if self.account_prefix.is_none() {
            self.account_prefix = other.account_prefix.clone();
        }
        if !self.emit_open_directives {
            self.emit_open_directives = other.emit_open_directives;
        }
        if self.open_date.is_none() {
            self.open_date = other.open_date.clone();
        }

        trace!("Merged output config: {:?}", self);
    }
}
