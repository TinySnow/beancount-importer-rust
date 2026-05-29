//! `Provider` 适配器抽象接口。
//!
//! 本模块定义了账单导入的统一供应商契约 [`Provider`]。
//! 该契约将导入过程拆分为两个阶段：
//! 1. `parse`: 输入文件 -> 标准化 [`RawRecord`](crate::model::data::raw_record::RawRecord) 列表。
//! 2. `transform`: `RawRecord` -> 领域交易 [`Transaction`](crate::model::transaction::Transaction)。
//!
//! 大多数供应商基于表格文件（CSV/XLSX），可直接调用 [`parse_tabular_source`]
//! 完成 `parse` 阶段的读取与字段映射。
//!
//! # 示例
//! ```rust,no_run
//! use std::path::Path;
//!
//! use beancount_importer_rust::{
//!     error::ImporterResult,
//!     interface::provider::{Provider, parse_tabular_source},
//!     model::{
//!         config::provider::ProviderConfig,
//!         data::raw_record::RawRecord,
//!         mapping::field_mapping::FieldMapping,
//!         rule::rule_engine::RuleEngine,
//!         transaction::Transaction,
//!     },
//! };
//!
//! struct DemoProvider;
//!
//! impl Provider for DemoProvider {
//!     fn name(&self) -> &'static str {
//!         "demo"
//!     }
//!
//!     fn parse(
//!         &self,
//!         path: &Path,
//!         mapping: &FieldMapping,
//!         config: &ProviderConfig,
//!         strict_mode: bool,
//!     ) -> ImporterResult<Vec<RawRecord>> {
//!         parse_tabular_source(path, mapping, config, strict_mode)
//!     }
//!
//!     fn transform(
//!         &self,
//!         _record: RawRecord,
//!         _rule_engine: &RuleEngine,
//!         _config: &ProviderConfig,
//!     ) -> ImporterResult<Option<Transaction>> {
//!         Ok(None)
//!     }
//! }
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use std::path::Path;

use crate::{
    error::ImporterResult,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    runtime::reader::tabular::TabularRecordReader,
};

/// 使用表格读取器解析源文件，供基于 CSV/XLSX 的供应商在 `parse()` 中复用。
///
/// 该函数封装了 `TabularRecordReader` 的初始化和读取逻辑，
/// 避免每个供应商重复实现相同的表格解析代码。
///
/// # 参数
/// - `path`: 输入文件路径（CSV/XLS/XLSX 等）。
/// - `mapping`: 字段映射规则。
/// - `config`: 供应商配置（含 tabular_options、skip_header_lines 等）。
/// - `strict_mode`: 严格模式开关。
pub fn parse_tabular_source(
    path: &Path,
    mapping: &FieldMapping,
    config: &ProviderConfig,
    strict_mode: bool,
) -> ImporterResult<Vec<RawRecord>> {
    let reader = TabularRecordReader::new(
        config.tabular_options.clone(),
        config.skip_header_lines,
        config.has_header_row,
        strict_mode,
    );

    reader.read_file(path, Some(mapping))
}

/// 供应商抽象接口。
///
/// 所有供应商实现通常位于 `src/providers/` 目录下，并由注册表统一发现与调度。
/// trait 约定本身是线程安全的（`Send + Sync`），便于在并发场景下复用。
pub trait Provider: Send + Sync {
    /// 返回供应商唯一标识（用于命令行与注册表检索）。
    ///
    /// 该值应保持稳定，避免与其它供应商重复。
    fn name(&self) -> &'static str;

    /// 返回供应商描述信息（用于日志、帮助信息与排错）。
    ///
    /// 默认值为 `"No description"`，实现方可按需覆盖。
    fn description(&self) -> &'static str {
        "No description"
    }

    /// 返回供应商显示名称（用于输出 `source` 元数据标签）。
    ///
    /// 默认回退到 [`name`](Self::name)，各实现应覆盖为中文等人类可读名称，
    /// 确保无 `--config` 时也能生成一致的 source 标签。
    fn display_name(&self) -> &'static str {
        self.name()
    }

    /// 将源数据文件解析为标准化原始记录列表。
    ///
    /// # 参数
    /// - `path`: 输入文件路径。
    /// - `mapping`: 字段映射规则，用于将源列映射到标准字段。
    /// - `config`: 供应商配置（含表格读取参数等）。
    /// - `strict_mode`: 严格模式开关。
    ///
    /// # 返回值
    /// 成功时返回 `Vec<RawRecord>`；失败时返回 `ImporterError`。
    ///
    /// # Errors
    /// 当文件读取、格式解析或字段映射失败时返回错误。
    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>>;

    /// 将一条标准化原始记录转换为一笔 Beancount 交易。
    ///
    /// # 参数
    /// - `record`: 由 `parse` 阶段产出的单条标准记录。
    /// - `rule_engine`: 规则引擎，可用于账户、元数据与忽略策略决策。
    /// - `config`: 供应商配置，用于读取默认账户、币种等转换参数。
    ///
    /// # 返回值
    /// - `Ok(Some(Transaction))`: 成功生成交易。
    /// - `Ok(None)`: 记录被有意忽略（例如命中 `ignore: true` 规则）。
    /// - `Err(_)`: 转换过程出现不可恢复错误。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::{
    ///     error::ImporterResult,
    ///     interface::provider::{Provider, parse_tabular_source},
    ///     model::{
    ///         account::{amount::Amount, posting::Posting},
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         mapping::field_mapping::FieldMapping,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    /// use chrono::NaiveDate;
    /// use rust_decimal::Decimal;
    /// use std::path::Path;
    ///
    /// struct DemoProvider;
    ///
    /// impl Provider for DemoProvider {
    ///     fn name(&self) -> &'static str {
    ///         "demo"
    ///     }
    ///
    ///     fn parse(
    ///         &self,
    ///         path: &Path,
    ///         mapping: &FieldMapping,
    ///         config: &ProviderConfig,
    ///         strict_mode: bool,
    ///     ) -> ImporterResult<Vec<RawRecord>> {
    ///         parse_tabular_source(path, mapping, config, strict_mode)
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         let date = record
    ///             .date
    ///             .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date"));
    ///         let amount = record.amount.unwrap_or(Decimal::ZERO);
    ///         let currency = record.currency.unwrap_or_else(|| "CNY".to_string());
    ///         let narration = record.narration.unwrap_or_else(|| "Imported".to_string());
    ///
    ///         let txn = Transaction::new(date, narration)
    ///             .with_flag('*')
    ///             .with_posting(Posting::new("Assets:Unknown").with_amount(Amount::new(amount, currency)))
    ///             .with_posting(Posting::new("Equity:Opening-Balances"));
    ///
    ///         Ok(Some(txn))
    ///     }
    /// }
    /// ```
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>>;
}
