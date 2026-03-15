//! 运行时主流程与配置装配。

mod config_loader;

use std::{
    fs,
    io::{self, Write},
};

use anyhow::{Context, Result, anyhow};
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

fn transform_records(
    provider: &dyn crate::interface::provider::Provider,
    raw_records: Vec<crate::model::data::raw_record::RawRecord>,
    rule_engine: &RuleEngine,
    provider_config: &crate::model::config::provider::ProviderConfig,
    strict_mode: bool,
) -> Result<Vec<Transaction>> {
    let mut success_count = 0usize;
    let mut ignored_count = 0usize;
    let mut error_count = 0usize;
    let mut transactions = Vec::new();

    for (index, raw_record) in raw_records.into_iter().enumerate() {
        match provider.transform(raw_record, rule_engine, provider_config) {
            Ok(Some(transaction)) => {
                success_count += 1;
                debug!(
                    "Record {} transformed: {} {}",
                    index + 1,
                    transaction.date,
                    transaction.narration
                );
                transactions.push(transaction);
            }
            Ok(None) => {
                ignored_count += 1;
                debug!("Record {} ignored by rule", index + 1);
            }
            Err(error) => {
                error_count += 1;

                if strict_mode {
                    return Err(anyhow!(
                        "Record {} transformation failed in strict mode: {}",
                        index + 1,
                        error
                    ));
                }

                warn!("Record {} skipped with error: {}", index + 1, error);
            }
        }
    }

    // 保持输出稳定，便于审阅与对比。
    transactions.sort_by_key(|tx| tx.date);

    info!(
        "Transformation complete: {} success, {} ignored, {} failed",
        success_count, ignored_count, error_count
    );

    Ok(transactions)
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{ImporterError, ImporterResult},
        interface::provider::Provider,
        model::{
            config::{global::GlobalConfig, provider::ProviderConfig},
            data::raw_record::RawRecord,
            mapping::field_mapping::FieldMapping,
            rule::{Rule, rule_engine::RuleEngine},
            transaction::Transaction,
        },
    };

    use super::transform_records;

    struct AlwaysFailProvider;

    impl Provider for AlwaysFailProvider {
        fn name(&self) -> &'static str {
            "always-fail"
        }

        fn parse(
            &self,
            _path: &std::path::Path,
            _mapping: &FieldMapping,
            _config: &ProviderConfig,
            _strict_mode: bool,
        ) -> ImporterResult<Vec<RawRecord>> {
            Ok(vec![])
        }

        fn transform(
            &self,
            _record: RawRecord,
            _rule_engine: &RuleEngine,
            _config: &ProviderConfig,
        ) -> ImporterResult<Option<Transaction>> {
            Err(ImporterError::Conversion("mock failure".to_string()))
        }
    }

    fn build_rule_engine() -> RuleEngine<'static> {
        let provider_rules: &'static [Rule] = Box::leak(Vec::<Rule>::new().into_boxed_slice());
        let global: &'static GlobalConfig = Box::leak(Box::new(GlobalConfig::default()));
        RuleEngine::new(provider_rules, global)
    }

    #[test]
    fn strict_mode_returns_error_on_transform_failure() {
        let provider = AlwaysFailProvider;
        let records = vec![RawRecord::new()];
        let rule_engine = build_rule_engine();
        let provider_config = ProviderConfig::default();

        let result = transform_records(&provider, records, &rule_engine, &provider_config, true);
        assert!(result.is_err());
    }

    #[test]
    fn non_strict_mode_skips_transform_failure() {
        let provider = AlwaysFailProvider;
        let records = vec![RawRecord::new()];
        let rule_engine = build_rule_engine();
        let provider_config = ProviderConfig::default();

        let result = transform_records(&provider, records, &rule_engine, &provider_config, false)
            .expect("non-strict mode should not fail");

        assert!(result.is_empty());
    }
}
