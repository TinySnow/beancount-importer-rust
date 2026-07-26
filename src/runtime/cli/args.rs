//! 命令行参数模型。
//!
//! 该模块定义了命令行入参结构 [`Cli`]，用于承载导入流程所需的
//! 数据源、配置路径、输出路径与日志行为。
//!
//! # 参数优先级
//! 日志级别按以下优先级计算（从高到低）：
//! 1. `--quiet` 固定为 `Error`
//! 2. `--verbose` 固定为 `Debug`
//! 3. `--log-level` 或其默认值
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::runtime::cli::Cli;
//! use clap::Parser;
//!
//! let cli = Cli::parse_from([
//!     "beancount-importer",
//!     "--provider",
//!     "wechat",
//!     "--source",
//!     "records.csv",
//! ]);
//!
//! assert_eq!(cli.provider.as_deref(), Some("wechat"));
//! assert_eq!(cli.source.as_deref().unwrap().to_string_lossy(), "records.csv");
//! assert_eq!(cli.config.to_string_lossy(), "config.yml");
//! ```

use clap::Parser;
use std::path::PathBuf;

use crate::runtime::cli::log_level::LogLevel;

/// Beancount 交易导入器命令行参数。
///
/// 该结构体通过 `clap` 的 `derive(Parser)` 自动完成参数解析与校验。
/// 可通过 [`Parser::parse`] 或 [`Parser::parse_from`] 构造实例。
///
/// # 示例
/// ```rust
/// use beancount_importer_rust::runtime::cli::Cli;
/// use clap::Parser;
///
/// let cli = Cli::parse_from([
///     "beancount-importer",
///     "--provider",
///     "alipay",
///     "--source",
///     "input.csv",
///     "--output",
///     "output.beancount",
///     "--log-level",
///     "info",
/// ]);
///
/// assert_eq!(cli.provider.as_deref(), Some("alipay"));
/// assert_eq!(cli.source.as_deref().unwrap().to_string_lossy(), "input.csv");
/// assert_eq!(cli.output.as_ref().unwrap().to_string_lossy(), "output.beancount");
/// ```
#[derive(Parser, Debug)]
#[command(name = "beancount-importer")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// 数据提供方标识（例如：`alipay`、`wechat`、`futu`、`icbc`）。
    ///
    /// 批量模式下无需提供。
    #[arg(short, long, required_unless_present = "batch")]
    pub provider: Option<String>,

    /// 数据源文件路径（CSV/Excel）。
    ///
    /// 批量模式下无需提供。
    #[arg(short, long, required_unless_present = "batch")]
    pub source: Option<PathBuf>,

    /// 供应商配置文件路径。
    ///
    /// 默认值为当前工作目录下的 `config.yml`。
    #[arg(short, long, default_value = "config.yml")]
    pub config: PathBuf,

    /// 全局配置文件路径（可选）。
    ///
    /// 设置后可覆盖 provider 之间共享的公共规则与账户配置。
    #[arg(short, long)]
    pub global_config: Option<PathBuf>,

    /// 字段映射文件路径（可选）。
    ///
    /// 用于覆盖内置 mapping；未设置时自动使用编译期内嵌映射。
    /// 支持 CSV 列名 -> 标准字段的显式映射配置。
    #[arg(short = 'm', long)]
    pub mapping: Option<PathBuf>,

    /// 输出文件路径（可选）。
    ///
    /// 未设置时默认输出到标准输出（stdout）。
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// 显式日志级别。
    ///
    /// 当未指定时默认为 [`LogLevel::Warn`]。
    #[arg(long, value_enum, default_value_t = LogLevel::Warn)]
    pub log_level: LogLevel,

    /// 静默模式，等同于最终日志级别 `Error`。
    ///
    /// 与 `--log-level` 冲突，由 `clap` 在解析阶段强制校验。
    #[arg(short, long, conflicts_with = "log_level")]
    pub quiet: bool,

    /// 详细模式，等同于最终日志级别 `Debug`。
    ///
    /// 与 `--log-level`、`--quiet` 均冲突，由 `clap` 在解析阶段强制校验。
    #[arg(short, long, conflicts_with_all = ["log_level", "quiet"])]
    pub verbose: bool,

    /// 严格模式。
    ///
    /// 开启后只要出现一条记录解析或转换失败，流程立即返回错误并退出。
    #[arg(long)]
    pub strict: bool,

    /// 批量导入模式（YAML 配置文件路径）。
    ///
    /// 指定后忽略 --provider/--source，从 batch 文件中读取多个导入任务并依次执行。
    /// 适用于月度多 provider 批量导入场景。
    #[arg(short, long, verbatim_doc_comment)]
    pub batch: Option<PathBuf>,
}

impl Cli {
    /// 计算最终生效的日志级别。
    ///
    /// 该方法封装了 CLI 参数之间的优先级规则，供初始化日志系统时统一使用。
    ///
    /// # 返回值
    /// 返回可直接用于 `log` / `env_logger` 初始化的 [`log::LevelFilter`]。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::runtime::cli::Cli;
    /// use clap::Parser;
    /// use log::LevelFilter;
    ///
    /// let quiet = Cli::parse_from([
    ///     "beancount-importer",
    ///     "--provider",
    ///     "alipay",
    ///     "--source",
    ///     "records.csv",
    ///     "--quiet",
    /// ]);
    /// assert_eq!(quiet.effective_log_level(), LevelFilter::Error);
    ///
    /// let verbose = Cli::parse_from([
    ///     "beancount-importer",
    ///     "--provider",
    ///     "alipay",
    ///     "--source",
    ///     "records.csv",
    ///     "--verbose",
    /// ]);
    /// assert_eq!(verbose.effective_log_level(), LevelFilter::Debug);
    /// ```
    pub fn effective_log_level(&self) -> log::LevelFilter {
        // 这里显式写成顺序分支，确保优先级语义一目了然：
        // quiet > verbose > explicit/default log_level。
        if self.quiet {
            log::LevelFilter::Error
        } else if self.verbose {
            log::LevelFilter::Debug
        } else {
            self.log_level.to_level_filter()
        }
    }
}
