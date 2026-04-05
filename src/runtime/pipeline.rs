//! 导入流水线编排模块。
//!
//! 该模块负责将供应商解析得到的原始记录批量转换为标准交易，并执行写出前后处理：
//! - 严格/非严格模式下的逐条转换与错误策略；
//! - 输出稳定排序；
//! - 基于库存种子补全卖出分录推断成本；
//! - 逐笔收益元数据（PnL）写入。
//!
//! 该阶段是“解析输入”与“最终写出”之间的桥接层。

use anyhow::{Result, anyhow};
use log::{debug, info, warn};

use crate::{
    interface::provider::Provider,
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        rule::rule_engine::RuleEngine, transaction::Transaction,
    },
};

use super::{
    inventory::{load_seed_inventory_from_files, resolve_inferred_cost_postings_with_inventory},
    pnl::annotate_trade_profit_metadata,
    sorting::sort_transactions_for_output,
};

/// 将供应商原始记录转换为 Beancount 交易，并执行后处理：
/// 稳定排序、卖出 lot 成本补全、PnL 元数据标注。
///
/// # 参数
/// - `provider`：当前供应商实现，负责单条记录转换；
/// - `raw_records`：待转换原始记录列表；
/// - `rule_engine`：规则引擎；
/// - `provider_config`：供应商配置；
/// - `strict_mode`：严格模式开关。
///
/// # 返回值
/// - `Ok(Vec<Transaction>)`：转换成功并完成后处理的交易列表；
/// - `Err`：严格模式下遇到首条转换错误时立即返回。
///
/// # 严格模式行为
/// - `strict_mode = true`：任一记录转换失败即终止并返回错误；
/// - `strict_mode = false`：失败记录记日志并跳过，其余记录继续处理。
pub(crate) fn transform_records(
    provider: &dyn Provider,
    raw_records: Vec<RawRecord>,
    rule_engine: &RuleEngine,
    provider_config: &ProviderConfig,
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
                    // 严格模式下保留首个失败上下文，避免继续处理掩盖问题根因。
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

    // 先做稳定排序，保证多次导入结果顺序一致，便于对账与比对。
    sort_transactions_for_output(&mut transactions);

    // 写出前补全推断成本（`{}`），避免 Beancount 卖出 lot 匹配产生歧义。
    let mut seed_inventory = load_seed_inventory_from_files(&provider_config.inventory_seed_files);
    resolve_inferred_cost_postings_with_inventory(&mut transactions, &mut seed_inventory);

    // 在 lot 补全后写入逐笔收益元数据：grossPnl / feeTotal / netPnl。
    annotate_trade_profit_metadata(&mut transactions);

    info!(
        "Transformation complete: {} success, {} ignored, {} failed",
        success_count, ignored_count, error_count
    );

    Ok(transactions)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

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

    /// 测试桩：任何 `transform` 调用都返回转换失败。
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

    /// 测试桩：任何 `transform` 调用都返回固定成功交易。
    struct AlwaysPassProvider;

    impl Provider for AlwaysPassProvider {
        fn name(&self) -> &'static str {
            "always-pass"
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
            Ok(Some(Transaction::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                "ok",
            )))
        }
    }

    /// 构建一个不带规则的最小可用规则引擎，供流水线单测复用。
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

    #[test]
    fn transform_pipeline_keeps_successful_records() {
        let provider = AlwaysPassProvider;
        let records = vec![RawRecord::new(), RawRecord::new()];
        let rule_engine = build_rule_engine();
        let provider_config = ProviderConfig::default();

        let result = transform_records(&provider, records, &rule_engine, &provider_config, true)
            .expect("transform should succeed");

        assert_eq!(result.len(), 2);
    }
}
