//! `Provider` 适配器抽象接口。
//!
//! 本模块定义了账单导入的统一供应商契约 [`Provider`]。
//! 该契约将导入过程拆分为两个阶段：
//! 1. `parse`: 输入文件 -> 标准化 [`RawRecord`](crate::model::data::raw_record::RawRecord) 列表。
//! 2. `transform`: `RawRecord` -> 领域交易 [`Transaction`](crate::model::transaction::Transaction)。
//!
//! 默认 `parse` 实现已经封装了 `TabularRecordReader` 的初始化与读取逻辑，
//! 因此大多数供应商仅需实现 `name` 与 `transform`。
//!
//! # 示例
//! ```rust,no_run
//! use std::path::Path;
//!
//! use beancount_importer_rust::{
//!     error::ImporterResult,
//!     interface::provider::Provider,
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
//!     fn transform(
//!         &self,
//!         _record: RawRecord,
//!         _rule_engine: &RuleEngine,
//!         _config: &ProviderConfig,
//!     ) -> ImporterResult<Option<Transaction>> {
//!         Ok(None)
//!     }
//! }
//!
//! # let provider = DemoProvider;
//! # let mapping = FieldMapping::default();
//! # let config = ProviderConfig::default();
//! // 复用 trait 提供的默认 parse 实现。
//! let _records = provider.parse(Path::new("statement.csv"), &mapping, &config, false)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use std::path::Path;

use crate::{
    error::ImporterResult,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, reader::tabular::TabularRecordReader,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
};

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

    /// 将源数据文件解析为标准化原始记录列表。
    ///
    /// # 参数
    /// - `path`: 输入文件路径（CSV/XLS/XLSX 等）。
    /// - `mapping`: 字段映射规则，用于将源列映射到标准字段。
    /// - `config`: 供应商配置（包含表格读取参数、跳过行数等）。
    /// - `strict_mode`: 严格模式开关，控制读取与映射过程的容错策略。
    ///
    /// # 返回值
    /// 成功时返回 `Vec<RawRecord>`；失败时返回 `ImporterError`。
    ///
    /// # 默认实现
    /// - 根据 `config` 构造 `TabularRecordReader`。
    /// - 调用 `read_file` 读取并应用 `mapping`。
    ///
    /// # Errors
    /// 当文件读取、编码解析、表格解析或字段映射失败时返回错误。
    ///
    /// # 示例
    /// ```rust,no_run
    /// use std::path::Path;
    ///
    /// use beancount_importer_rust::{
    ///     error::ImporterResult,
    ///     interface::provider::Provider,
    ///     model::{
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         mapping::field_mapping::FieldMapping,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    ///
    /// struct DemoProvider;
    ///
    /// impl Provider for DemoProvider {
    ///     fn name(&self) -> &'static str {
    ///         "demo"
    ///     }
    ///
    ///     fn transform(
    ///         &self,
    ///         _record: RawRecord,
    ///         _rule_engine: &RuleEngine,
    ///         _config: &ProviderConfig,
    ///     ) -> ImporterResult<Option<Transaction>> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// let provider = DemoProvider;
    /// let mapping = FieldMapping::default();
    /// let config = ProviderConfig::default();
    ///
    /// let _records = provider.parse(Path::new("statement.csv"), &mapping, &config, true)?;
    /// # Ok::<(), beancount_importer_rust::error::ImporterError>(())
    /// ```
    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        // 解析器参数统一从 ProviderConfig 注入，避免不同供应商重复构造读取逻辑。
        let reader = TabularRecordReader::new(
            config.tabular_options.clone(),
            config.skip_header_lines,
            config.has_header_row,
            strict_mode,
        );

        // 传入映射并执行读取：输出统一的 RawRecord，供后续规则与交易转换阶段消费。
        reader.read_file(path, Some(mapping))
    }

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
    ///     interface::provider::Provider,
    ///     model::{
    ///         account::{amount::Amount, posting::Posting},
    ///         config::provider::ProviderConfig,
    ///         data::raw_record::RawRecord,
    ///         rule::rule_engine::RuleEngine,
    ///         transaction::Transaction,
    ///     },
    /// };
    /// use chrono::NaiveDate;
    /// use rust_decimal::Decimal;
    ///
    /// struct DemoProvider;
    ///
    /// impl Provider for DemoProvider {
    ///     fn name(&self) -> &'static str {
    ///         "demo"
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
