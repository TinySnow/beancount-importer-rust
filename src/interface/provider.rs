//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
use std::path::Path;

use crate::{
    error::ImporterResult,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, reader::csv_reader::CsvRecordReader,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
};

/// 供应商抽象接口。
///
/// 所有供应商实现都位于 `providers/` 目录下。
pub trait Provider: Send + Sync {
    /// 供应商唯一标识（用于命令行与注册表检索）。
    fn name(&self) -> &'static str;

    /// 供应商描述信息（用于日志与排错）。
    fn description(&self) -> &'static str {
        "No description"
    }

    /// 将源数据文件解析为标准化原始记录。
    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        let reader = CsvRecordReader::new(
            config.csv_options.clone(),
            config.skip_header_lines,
            config.has_csv_header,
            strict_mode,
        );

        reader.read_file(path, Some(mapping))
    }

    /// 将一条标准化原始记录转换为一笔 Beancount 交易。
    ///
    /// 当记录需要被有意忽略时返回 `Ok(None)`（例如，
    /// 命中 `ignore: true` 规则）。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>>;
}
