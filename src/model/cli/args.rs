//! 模块说明：命令行参数模型与日志级别解析。
//!
//! 文件路径：src/model/cli/args.rs。
//! 该文件围绕 'args' 的职责提供实现。
//! 关键符号：Cli、effective_log_level。

use clap::Parser;
use std::path::PathBuf;

use crate::model::cli::log_level::LogLevel;

/// Beancount 交易导入器
///
/// 从各种金融机构的对账单生成 Beancount 格式的记账文件
#[derive(Parser, Debug)]
#[command(name = "beancount-importer")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// 供应商/银行名称（例如：`alipay`、`wechat`、`futu`、`icbc`）
    #[arg(short, long)]
    pub provider: String,

    /// 数据源文件路径（CSV/Excel）
    #[arg(short, long)]
    pub source: PathBuf,

    /// 供应商配置文件路径
    #[arg(short, long, default_value = "config.yml")]
    pub config: PathBuf,

    /// 全局配置文件路径
    #[arg(short, long)]
    pub global_config: Option<PathBuf>,

    /// 输出文件路径（默认输出到标准输出）
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// 日志级别
    #[arg(long, value_enum, default_value_t = LogLevel::Warn)]
    pub log_level: LogLevel,

    /// 静默模式，等同于 `--log-level=error`
    #[arg(short, long, conflicts_with = "log_level")]
    pub quiet: bool,

    /// 详细模式，等同于 `--log-level=debug`
    #[arg(short, long, conflicts_with_all = ["log_level", "quiet"])]
    pub verbose: bool,

    /// 严格模式：解析或转换任意一条记录失败时立即失败退出
    #[arg(long)]
    pub strict: bool,
}

impl Cli {
    /// 获取最终的日志级别
    pub fn effective_log_level(&self) -> log::LevelFilter {
        if self.quiet {
            log::LevelFilter::Error
        } else if self.verbose {
            log::LevelFilter::Debug
        } else {
            self.log_level.to_level_filter()
        }
    }
}
