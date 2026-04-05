//! 输出配置模型。
//!
//! 本模块定义写出 Beancount 文本时的格式与行为开关，并提供
//! “供应商配置覆盖全局配置”的合并逻辑。
//!
//! # 合并语义
//! `OutputConfig::merge_with` 采用“当前配置优先、全局配置兜底”的策略：
//! - 当前值是默认值或空值时，才会继承另一份配置。
//! - 当前值已显式设置时，保持不变。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::config::output::OutputConfig;
//!
//! let mut provider_output = OutputConfig::default();
//! provider_output.account_prefix = Some("Assets:Broker:Galaxy".to_string());
//!
//! let global_output = OutputConfig {
//!     date_format: "%d/%m/%Y".to_string(),
//!     decimal_places: 4,
//!     account_prefix: Some("Assets:Global".to_string()),
//!     emit_open_directives: true,
//!     open_date: Some("2025-01-01".to_string()),
//!     booking_method: Some("FIFO".to_string()),
//! };
//!
//! provider_output.merge_with(&global_output);
//! assert_eq!(provider_output.date_format, "%d/%m/%Y");
//! assert_eq!(provider_output.decimal_places, 4);
//! assert_eq!(
//!     provider_output.account_prefix.as_deref(),
//!     Some("Assets:Broker:Galaxy")
//! );
//! assert!(provider_output.emit_open_directives);
//! assert_eq!(provider_output.open_date.as_deref(), Some("2025-01-01"));
//! assert_eq!(provider_output.booking_method.as_deref(), Some("FIFO"));
//! ```

use log::trace;
use serde::{Deserialize, Serialize};

/// 输出格式配置。
///
/// 该结构体可在全局层和供应商层分别定义，最终通过合并得到生效值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// 交易写出时使用的日期格式。
    #[serde(default = "default_date_format")]
    pub date_format: String,

    /// 金额格式化时使用的小数位数。
    #[serde(default = "default_decimal_places")]
    pub decimal_places: u32,

    /// 可选账户前缀。
    ///
    /// 常用于将同一份映射输出到不同账本命名空间。
    pub account_prefix: Option<String>,

    /// 是否为当前输出中的所有账户写出 `open` 指令。
    #[serde(default)]
    pub emit_open_directives: bool,

    /// 可选 `open` 日期，格式为 `%Y-%m-%d`。
    /// 若未配置且启用 `emit_open_directives`，则使用最早交易日期。
    pub open_date: Option<String>,

    /// 可选库存账户 lot 匹配方法（用于 `open` 指令）。
    ///
    /// 示例值：`STRICT`、`FIFO`、`LIFO`、`AVERAGE`、`NONE`。
    /// 若配置，writer 会为含非货币持仓的账户输出：
    /// `YYYY-MM-DD open <Account> "<BookingMethod>"`。
    pub booking_method: Option<String>,
}

/// `date_format` 字段默认值工厂函数。
fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

/// `decimal_places` 字段默认值工厂函数。
fn default_decimal_places() -> u32 {
    2
}

impl Default for OutputConfig {
    /// 创建输出配置默认实例。
    fn default() -> Self {
        Self {
            date_format: default_date_format(),
            decimal_places: default_decimal_places(),
            account_prefix: None,
            emit_open_directives: false,
            open_date: None,
            booking_method: None,
        }
    }
}

impl OutputConfig {
    /// 与另一份输出配置合并（当前配置优先）。
    ///
    /// 当当前配置字段仍为默认值或空值时，从 `other` 继承。
    /// 该方法通常用于“供应商配置 + 全局默认配置”的合并。
    ///
    /// # 参数
    /// - `other`：通常为全局输出配置。
    pub fn merge_with(&mut self, other: &OutputConfig) {
        trace!("Merging output config with global defaults");

        // `date_format` 和 `decimal_places` 通过“是否仍为默认值”判断是否覆盖。
        if self.date_format == default_date_format() {
            self.date_format = other.date_format.clone();
        }
        if self.decimal_places == default_decimal_places() {
            self.decimal_places = other.decimal_places;
        }
        // 可选字段仅在当前未显式设置时继承。
        if self.account_prefix.is_none() {
            self.account_prefix = other.account_prefix.clone();
        }
        // 布尔开关采用“已有 true 不回退”的策略。
        if !self.emit_open_directives {
            self.emit_open_directives = other.emit_open_directives;
        }
        if self.open_date.is_none() {
            self.open_date = other.open_date.clone();
        }
        if self.booking_method.is_none() {
            self.booking_method = other.booking_method.clone();
        }

        trace!("Merged output config: {:?}", self);
    }
}
