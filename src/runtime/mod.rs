//! 运行时编排入口模块。
//!
//! 该模块负责串联导入执行链路：加载配置、调用供应商解析、按规则转换交易、
//! 执行排序/库存/PnL 后处理，并最终写出 Beancount 文本。
//!
//! 运行时子模块职责：
//! - `config_loader`：读取并组装运行配置；
//! - `pipeline`：执行记录转换与后处理编排；
//! - `sorting`：写出前确定性排序；
//! - `inventory`：基于库存信息补全推断成本；
//! - `pnl`：写入逐笔收益元数据；
//! - `cli`：命令行参数与日志级别模型；
//! - `reader`：表格读取与映射实现；
//! - `writer`：Beancount 文本写出实现；
//! - `provider_registry`：供应商注册与检索。

/// 命令行参数与日志级别定义。
///
/// 放在 runtime 层，避免将“进程入口参数”混入纯领域模型层。
pub mod cli;
mod config_loader;
mod inventory;
mod pipeline;
mod pnl;
pub mod provider_registry;
pub mod reader;
mod sorting;
pub mod writer;

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use log::{debug, info, warn};
use serde::Deserialize;

use self::cli::Cli;
use crate::{
    error::{ImporterError, ImporterResult},
    model::rule::rule_engine::RuleEngine,
    runtime::{
        config_loader::load, pipeline::transform_records, provider_registry::ProviderRegistry,
        writer::beancount_writer::BeancountWriter,
    },
};

// ---------------------------------------------------------------------------
// 批量导入模式
// ---------------------------------------------------------------------------

/// 批量导入任务配置（对应 `batch.yml` 顶层结构）。
#[derive(Debug, Deserialize)]
struct BatchConfig {
    /// 任务列表
    #[serde(default)]
    imports: Vec<BatchItem>,
    /// 全局日志级别覆盖（可选，等同于 --log-level）
    #[serde(default)]
    log_level: Option<String>,
    /// 全局严格模式覆盖（可选，等同于 --strict）
    #[serde(default)]
    strict: bool,
}

/// 批量导入中的单条任务。
#[derive(Debug, Deserialize)]
struct BatchItem {
    /// 数据提供方标识
    provider: String,
    /// 数据源文件路径（相对于 batch 文件所在目录）
    source: PathBuf,
    /// 供应商配置文件路径（相对于 batch 文件所在目录，可选）
    config: Option<PathBuf>,
    /// 字段映射文件路径（相对于 batch 文件所在目录，可选）
    #[serde(alias = "mapping")]
    mapping_file: Option<PathBuf>,
    /// 输出文件路径（相对于 batch 文件所在目录，可选）
    output: Option<PathBuf>,
    /// 全局配置文件路径（可选）
    global_config: Option<PathBuf>,
    /// 逐任务严格模式覆盖（可选）
    #[serde(default)]
    strict: Option<bool>,
}

/// 批量导入入口。
///
/// 读取 `batch.yml`，依次执行每个导入任务。
/// 路径在 batch 文件所在目录下解析。
pub fn run_batch(batch_path: &Path) -> ImporterResult<()> {
    let batch_dir = batch_path.parent().unwrap_or_else(|| Path::new("."));
    let content = fs::read_to_string(batch_path).map_err(|e| {
        ImporterError::Io(e).with_context(format!("Failed to read batch file: {}", batch_path.display()))
    })?;
    let batch: BatchConfig = serde_yaml::from_str(&content).map_err(|e| {
        ImporterError::Yaml(e).with_context("Invalid batch YAML".to_string())
    })?;

    let batch_strict = batch.strict;
    let total = batch.imports.len();
    info!("Batch mode: {} import(s)", total);

    // Batch 级 log_level 覆盖
    let log_level = match batch.log_level.as_deref() {
        Some("error") => crate::runtime::cli::log_level::LogLevel::Error,
        Some("info")  => crate::runtime::cli::log_level::LogLevel::Info,
        Some("debug") => crate::runtime::cli::log_level::LogLevel::Debug,
        Some("trace") => crate::runtime::cli::log_level::LogLevel::Trace,
        _             => crate::runtime::cli::log_level::LogLevel::Warn,
    };

    for (i, item) in batch.imports.iter().enumerate() {
        info!("[{}/{}] provider={} source={}", i + 1, total, item.provider, item.source.display());

        let resolve = |p: &Path| -> PathBuf {
            if p.is_absolute() { p.to_path_buf() } else { batch_dir.join(p) }
        };

        let source = resolve(&item.source);

        // config: 优先显式路径，否则默认 batch_dir/config.yml
        let config = item.config.as_ref()
            .map(|p| resolve(p))
            .unwrap_or_else(|| batch_dir.join("config.yml"));

        let global_config = item.global_config.as_ref().map(|p| resolve(p));
        let mapping = item.mapping_file.as_ref().map(|p| resolve(p));
        let output = item.output.as_ref().map(|p| resolve(p));
        let strict = item.strict.unwrap_or(batch_strict);

        let cli = Cli {
            provider: item.provider.clone(),
            source,
            config,
            global_config,
            mapping,
            output,
            log_level,
            quiet: false,
            verbose: false,
            strict,
            batch: None,
        };

        if let Err(e) = run(cli) {
            warn!("[{}/{}] {}: FAILED — {}", i + 1, total, item.provider, e);
            return Err(e);
        }
        info!("[{}/{}] {}: OK", i + 1, total, item.provider);
    }

    info!("Batch complete: {} import(s)", total);
    Ok(())
}

// ---------------------------------------------------------------------------
// 单次导入
// ---------------------------------------------------------------------------
///
/// 该函数是整个导入流程的主入口，负责协调各个模块完成从源文件到 Beancount 格式的转换。
///
/// # 流程步骤
/// 1. 加载 global/provider/mapping 配置
/// 2. 调用供应商解析源记录
/// 3. 转换为标准交易
/// 4. 输出 Beancount 文本到文件或标准输出
///
/// # 参数
/// - `cli`：命令行参数对象，包含供应商、源文件、配置文件等信息
///
/// # 返回值
/// - `Ok(())`：导入流程成功完成
/// - `Err(Error)`：导入流程失败，包含详细错误信息
///
/// # 错误处理
/// - 配置加载失败
/// - 供应商不存在或解析失败
/// - 记录转换失败
/// - 输出文件创建失败
///
/// # 日志输出
/// - 启动和完成信息
/// - 解析的记录数量
/// - 生成的交易数量
/// - 调试信息（供应商、文件路径等）
pub fn run(cli: Cli) -> ImporterResult<()> {
    // 输出启动信息
    info!("Starting beancount-importer");
    debug!("Provider: {}", cli.provider);
    debug!("Source file: {}", cli.source.display());
    debug!("Config file: {}", cli.config.display());

    // 先验证供应商是否存在，避免后续 mapping 加载错误遮盖真实原因。
    let registry = ProviderRegistry::global();
    let provider = registry
        .get(&cli.provider)
        .ok_or_else(|| {
            ImporterError::ProviderNotFound(format!(
                "Unknown provider '{}'. Available providers: {:?}",
                cli.provider,
                registry.list_providers()
            ))
        })?;

    // 加载配置文件
    let loaded = load(&cli)?;

    // 输出使用的供应商信息
    info!(
        "Using provider: {} ({})",
        provider.name(),
        provider.description()
    );

    // 解析源文件记录
    let raw_records = provider
        .parse(&cli.source, &loaded.mapping, &loaded.provider, cli.strict)
        .map_err(|e| e.with_context(format!("Failed to parse source file: {}", cli.source.display())))?;

    // 输出解析的记录数量
    info!("Parsed {} records", raw_records.len());

    // 初始化规则引擎并转换记录
    let rule_engine = RuleEngine::new(&loaded.provider.rules, &loaded.global);
    let transactions = transform_records(
        provider.as_ref(),
        raw_records,
        &rule_engine,
        &loaded.provider,
        cli.strict,
    )?;

    // 创建输出目标（文件或标准输出）
    let writer = BeancountWriter::new(loaded.provider.output.clone());
    let mut output: Box<dyn Write> = match cli.output {
        Some(path) => {
            info!("Writing output to file: {}", path.display());
            Box::new(
                fs::File::create(&path).map_err(|e| {
                    ImporterError::Io(e)
                        .with_context(format!("Failed to create output file: {}", path.display()))
                })?,
            )
        }
        None => {
            debug!("Writing output to stdout");
            Box::new(io::stdout())
        }
    };

    // 写入交易数据
    writer.write(&transactions, &mut output)?;
    info!("Successfully generated {} transactions", transactions.len());

    Ok(())
}
