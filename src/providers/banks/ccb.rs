//! 中国建设银行（CCB）账单导入适配器。
//!
//! 该模块定义了 `ccb` 供应商的最小实现：
//! - 提供稳定的供应商标识与描述；
//! - 传入银行特定的默认账户参数；
//! - 将转换流程委托给共享现金流转换器，避免各银行重复实现转换细节。
//!
//! # 示例
//! ```rust,no_run
//! use beancount_importer_rust::{
//!     interface::provider::Provider,
//!     model::{
//!         config::{global::GlobalConfig, provider::ProviderConfig},
//!         data::raw_record::RawRecord,
//!         rule::{Rule, rule_engine::RuleEngine},
//!     },
//!     providers::banks::ccb::CcbProvider,
//! };
//!
//! let provider = CcbProvider;
//! assert_eq!(provider.name(), "ccb");
//! assert_eq!(provider.description(), "CCB statement importer");
//!
//! let config = ProviderConfig::default();
//! let global = GlobalConfig::default();
//! let provider_rules: [Rule; 0] = [];
//! let rule_engine = RuleEngine::new(&provider_rules, &global);
//! let record = RawRecord::new();
//!
//! // 示例仅验证调用方式；真实导入时 record 需包含日期、金额等必要字段。
//! let _ = provider.transform(record, &rule_engine, &config)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use std::path::Path;

use crate::{
    error::ImporterResult,
    interface::provider::{Provider, parse_tabular_source},
    model::{
        config::provider::ProviderConfig, data::raw_record::RawRecord,
        mapping::field_mapping::FieldMapping, rule::rule_engine::RuleEngine,
        transaction::Transaction,
    },
    providers::shared::{CashflowTransformOptions, transform_cashflow_record},
};

/// CCB 现金流转换参数。
///
/// - `default_asset_fallback` 在规则和配置都未给出资产账户时兜底使用。
const CCB_OPTIONS: CashflowTransformOptions = CashflowTransformOptions {
    default_asset_fallback: "Assets:CCB",
};

/// CCB 账单 `Provider`。
///
/// 该类型为 `unit struct`，不持有运行时状态；
/// 转换依赖全部由 `transform` 方法参数注入。
pub struct CcbProvider;

impl Provider for CcbProvider {
    /// 返回供应商唯一标识：`"ccb"`。
    fn name(&self) -> &'static str {
        "ccb"
    }

    /// 返回供应商说明文本。
    fn description(&self) -> &'static str {
        "CCB statement importer"
    }

    fn display_name(&self) -> &'static str {
        "建设银行"
    }

    fn parse(
        &self,
        path: &Path,
        mapping: &FieldMapping,
        config: &ProviderConfig,
        strict_mode: bool,
    ) -> ImporterResult<Vec<RawRecord>> {
        parse_tabular_source(path, mapping, config, strict_mode)
    }

    /// 将一条标准化原始记录转换为交易。
    ///
    /// 关键逻辑：当前实现只负责注入 CCB 专属参数，
    /// 实际字段解析、收支分类、分录构建与元数据附加由共享转换器完成。
    fn transform(
        &self,
        record: RawRecord,
        rule_engine: &RuleEngine,
        config: &ProviderConfig,
    ) -> ImporterResult<Option<Transaction>> {
        // 统一走共享现金流转换流程，保证银行类 Provider 行为一致。
        transform_cashflow_record(
            self.name(),
            self.display_name(),
            CCB_OPTIONS,
            record,
            rule_engine,
            config,
        )
    }
}
