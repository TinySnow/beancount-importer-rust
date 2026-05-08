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
};

use anyhow::{Context, Result};
use log::{debug, info};

use self::cli::Cli;
use crate::{
    model::rule::rule_engine::RuleEngine,
    runtime::{
        config_loader::load, pipeline::transform_records, provider_registry::ProviderRegistry,
        writer::beancount_writer::BeancountWriter,
    },
};

/// 执行端到端导入流程
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
pub fn run(cli: Cli) -> Result<()> {
    // 输出启动信息
    info!("Starting beancount-importer");
    debug!("Provider: {}", cli.provider);
    debug!("Source file: {}", cli.source.display());
    debug!("Config file: {}", cli.config.display());

    // 先验证供应商是否存在，避免后续 mapping 加载错误遮盖真实原因。
    let registry = ProviderRegistry::global();
    let provider = registry.get(&cli.provider).with_context(|| {
        format!(
            "Unknown provider '{}'. Available providers: {:?}",
            cli.provider,
            registry.list_providers()
        )
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
        .with_context(|| format!("Failed to parse source file: {}", cli.source.display()))?;

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
                fs::File::create(&path)
                    .with_context(|| format!("Failed to create output file: {}", path.display()))?,
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
