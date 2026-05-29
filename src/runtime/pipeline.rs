//! 导入流水线编排模块。
//!
//! 该模块负责将供应商解析得到的原始记录批量转换为标准交易，并执行写出前后处理：
//! - 严格/非严格模式下的逐条转换与错误策略；
//! - 输出稳定排序；
//! - 基于库存种子补全卖出分录推断成本；
//! - 逐笔收益元数据（PnL）写入。
//!
//! 后处理步骤以可插拔的 `PipelineStage` 组织，可按需跳过（例如银行类供应商无需 PnL 阶段）。
//! 转换步骤使用迭代器惰性求值，避免批量收集带来的额外内存开销。

use log::{debug, info, warn};

use crate::{
    error::{ImporterError, ImporterResult},
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

/// 后处理阶段函数签名：接收交易切片原地修改。
pub type PipelineStage = fn(&mut Vec<Transaction>, &ProviderConfig);

/// 导入流水线，持有可插拔的后处理阶段列表。
pub struct Pipeline {
    stages: Vec<PipelineStage>,
}

impl Pipeline {
    /// 创建包含全部后处理阶段的默认流水线。
    ///
    /// 默认包含：稳定排序、库存补全（FIFO lot 匹配）、PnL 元数据标注。
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            stages: vec![
                sort_stage,
                inventory_stage,
                pnl_stage,
            ],
        }
    }

    /// 创建仅包含排序阶段的轻量流水线（适用于银行/第三方支付类供应商）。
    #[allow(dead_code)]
    pub fn cashflow_only() -> Self {
        Self {
            stages: vec![sort_stage],
        }
    }

    /// 添加一个后处理阶段。
    #[allow(dead_code)]
    pub fn add_stage(&mut self, stage: PipelineStage) {
        self.stages.push(stage);
    }

    /// 移除匹配名称的阶段（用于禁用某个后处理）。
    /// 此处通过比较函数指针地址判断，仅用于有条件跳过。
    #[allow(dead_code)]
    pub fn without(mut self, excluded: PipelineStage) -> Self {
        self.stages.retain(|&s| s as usize != excluded as usize);
        self
    }

    /// 执行流水线：转换原始记录 -> 应用后处理阶段 -> 返回交易列表。
    ///
    /// 转换步骤使用迭代器，惰性求值以避免中间分配。
    pub fn run(
        &self,
        provider: &dyn Provider,
        raw_records: Vec<RawRecord>,
        rule_engine: &RuleEngine,
        provider_config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<Transaction>> {
        let transform_iter = TransformIter {
            provider,
            records: raw_records.into_iter(),
            rule_engine,
            config: provider_config,
            strict_mode,
            index: 0,
            errored: false,
        };

        let mut success_count = 0usize;
        let mut ignored_count = 0usize;
        let mut error_count = 0usize;
        let mut transactions = Vec::new();

        for result in transform_iter {
            match result {
                Ok(Some(transaction)) => {
                    success_count += 1;
                    debug!(
                        "Record {} transformed: {} {}",
                        success_count,
                        transaction.date,
                        transaction.narration
                    );
                    transactions.push(transaction);
                }
                Ok(None) => {
                    ignored_count += 1;
                }
                Err(error) => {
                    error_count += 1;
                    if strict_mode {
                        return Err(error);
                    }
                    warn!("Record skipped with error: {}", error);
                }
            }
        }

        for stage in &self.stages {
            stage(&mut transactions, provider_config);
        }

        info!(
            "Transformation complete: {} success, {} ignored, {} failed",
            success_count, ignored_count, error_count
        );

        Ok(transactions)
    }
}

/// 转换迭代器：惰性遍历原始记录并逐条转换为交易。
///
/// 严格模式下，遇到第一个转换错误即停止迭代。
struct TransformIter<'a> {
    provider: &'a dyn Provider,
    records: std::vec::IntoIter<RawRecord>,
    rule_engine: &'a RuleEngine<'a>,
    config: &'a ProviderConfig,
    strict_mode: bool,
    index: usize,
    errored: bool,
}

impl<'a> Iterator for TransformIter<'a> {
    type Item = ImporterResult<Option<Transaction>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored && self.strict_mode {
            return None;
        }
        self.records.next().map(|raw_record| {
            self.index += 1;
            self.provider.transform(raw_record, self.rule_engine, self.config)
                .map_err(|error| {
                    self.errored = true;
                    ImporterError::Conversion(format!(
                        "Record {} transformation failed in strict mode: {}",
                        self.index,
                        error
                    ))
                })
        })
    }
}

fn sort_stage(transactions: &mut Vec<Transaction>, _config: &ProviderConfig) {
    sort_transactions_for_output(transactions);
}

fn inventory_stage(transactions: &mut Vec<Transaction>, config: &ProviderConfig) {
    if config.inventory_seed_files.is_empty() {
        return;
    }
    let mut seed_inventory = load_seed_inventory_from_files(&config.inventory_seed_files);
    resolve_inferred_cost_postings_with_inventory(transactions, &mut seed_inventory);
}

fn pnl_stage(transactions: &mut Vec<Transaction>, _config: &ProviderConfig) {
    annotate_trade_profit_metadata(transactions);
}

/// 便捷入口：创建默认流水线并运行。
pub(crate) fn transform_records(
    provider: &dyn Provider,
    raw_records: Vec<RawRecord>,
    rule_engine: &RuleEngine,
    provider_config: &ProviderConfig,
    strict_mode: bool,
) -> ImporterResult<Vec<Transaction>> {
    let pipeline = Pipeline::default();
    pipeline.run(provider, raw_records, rule_engine, provider_config, strict_mode)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use std::path::Path;

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
            _path: &Path,
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
            _path: &Path,
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
