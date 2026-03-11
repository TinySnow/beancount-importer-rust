//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
mod config_loader;

use std::{
    fs,
    io::{self, Write},
};

use anyhow::{Context, Result};
use log::{debug, info, warn};

use crate::{
    model::{
        cli::Cli, registry::provider_registry::ProviderRegistry, rule::rule_engine::RuleEngine,
        transaction::Transaction, writer::beancount_writer::BeancountWriter,
    },
    runtime::config_loader::load,
};

/// 运行导入主流程。
pub fn run(cli: Cli) -> Result<()> {
    info!("Starting beancount-importer");
    debug!("Provider: {}", cli.provider);
    debug!("Source file: {}", cli.source.display());
    debug!("Config file: {}", cli.config.display());

    let loaded = load(&cli)?;

    let registry = ProviderRegistry::global();
    let provider = registry.get(&cli.provider).with_context(|| {
        format!(
            "Unknown provider '{}'. Available providers: {:?}",
            cli.provider,
            registry.list_providers()
        )
    })?;

    info!(
        "Using provider: {} ({})",
        provider.name(),
        provider.description()
    );

    let raw_records = provider
        .parse(&cli.source, &loaded.mapping, &loaded.provider)
        .with_context(|| format!("Failed to parse source file: {}", cli.source.display()))?;

    info!("Parsed {} records", raw_records.len());

    let rule_engine = RuleEngine::new(&loaded.provider.rules, &loaded.global);
    let transactions = transform_records(
        provider.as_ref(),
        raw_records,
        &rule_engine,
        &loaded.provider,
    );

    let writer = BeancountWriter::new(loaded.provider.output.clone());
    let mut output: Box<dyn Write> = match cli.output {
        Some(path) => {
            info!("Writing output to file: {}", path.display());
            Box::new(
                fs::File::create(&path)
                    .with_context(|| format!("Failed to create output file: {}", path.display()))?,
            )
        }
        None => {
            debug!("Writing output to stdout");
            Box::new(io::stdout())
        }
    };

    writer.write(&transactions, &mut output)?;
    info!("Successfully generated {} transactions", transactions.len());

    Ok(())
}

fn transform_records(
    provider: &dyn crate::interface::provider::Provider,
    raw_records: Vec<crate::model::data::raw_record::RawRecord>,
    rule_engine: &RuleEngine,
    provider_config: &crate::model::config::provider::ProviderConfig,
) -> Vec<Transaction> {
    let mut success_count = 0usize;
    let mut ignored_count = 0usize;
    let mut error_count = 0usize;

    let mut transactions: Vec<_> = raw_records
        .into_iter()
        .enumerate()
        .filter_map(|(index, raw_record)| {
            match provider.transform(raw_record, rule_engine, provider_config) {
                Ok(Some(transaction)) => {
                    success_count += 1;
                    debug!(
                        "Record {} transformed: {} {}",
                        index + 1,
                        transaction.date,
                        transaction.narration
                    );
                    Some(transaction)
                }
                Ok(None) => {
                    ignored_count += 1;
                    debug!("Record {} ignored by rule", index + 1);
                    None
                }
                Err(error) => {
                    error_count += 1;
                    warn!("Record {} skipped with error: {}", index + 1, error);
                    None
                }
            }
        })
        .collect();

    // 保持输出稳定，便于审阅与比对。
    transactions.sort_by_key(|tx| tx.date);

    info!(
        "Transformation complete: {} success, {} ignored, {} failed",
        success_count, ignored_count, error_count
    );

    transactions
}
