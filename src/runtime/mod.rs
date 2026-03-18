//! 运行时导入流程入口与编排层。
mod config_loader;
mod currency;
mod inventory;
mod pipeline;
mod pnl;
mod sorting;

use std::{
    fs,
    io::{self, Write},
};

use anyhow::{Context, Result};
use log::{debug, info};

use crate::{
    model::{
        cli::Cli, registry::provider_registry::ProviderRegistry, rule::rule_engine::RuleEngine,
        writer::beancount_writer::BeancountWriter,
    },
    runtime::{config_loader::load, pipeline::transform_records},
};

/// 执行端到端导入流程：
///
/// 1. 加载 global/provider/mapping 配置；
/// 2. 调用供应商解析源记录；
/// 3. 转换为标准交易；
/// 4. 输出 Beancount 文本到文件或标准输出。
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
        .parse(&cli.source, &loaded.mapping, &loaded.provider, cli.strict)
        .with_context(|| format!("Failed to parse source file: {}", cli.source.display()))?;

    info!("Parsed {} records", raw_records.len());

    let rule_engine = RuleEngine::new(&loaded.provider.rules, &loaded.global);
    let transactions = transform_records(
        provider.as_ref(),
        raw_records,
        &rule_engine,
        &loaded.provider,
        cli.strict,
    )?;

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
