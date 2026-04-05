//! 命令行模型模块。
//!
//! 该模块聚合了命令行参数模型与日志级别模型：
//! - [`args`]：定义 CLI 参数结构 [`Cli`]
//! - [`log_level`]：定义日志级别枚举及转换逻辑
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::model::cli::{log_level::LogLevel, Cli};
//! use clap::Parser;
//! use log::LevelFilter;
//!
//! let cli = Cli::parse_from([
//!     "beancount-importer",
//!     "--provider",
//!     "alipay",
//!     "--source",
//!     "records.csv",
//!     "--verbose",
//! ]);
//!
//! assert_eq!(cli.effective_log_level(), LevelFilter::Debug);
//! assert_eq!(LogLevel::Warn.to_level_filter(), LevelFilter::Warn);
//! ```

pub mod args;
pub mod log_level;

pub use args::Cli;
