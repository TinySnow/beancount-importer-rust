//! 日志级别模型。
//!
//! 该模块定义了 CLI 使用的日志级别枚举 [`LogLevel`]，并提供到
//! `log` 生态标准类型 [`log::LevelFilter`] 的转换函数。
//!
//! # 示例
//! ```rust
//! use beancount_importer_rust::runtime::cli::log_level::LogLevel;
//! use log::LevelFilter;
//!
//! assert_eq!(LogLevel::Warn.to_level_filter(), LevelFilter::Warn);
//! assert_eq!(LogLevel::Trace.to_level_filter(), LevelFilter::Trace);
//! ```

use clap::ValueEnum;

/// 命令行可选的日志级别。
///
/// 该枚举实现了 [`ValueEnum`]，可直接作为 `clap` 参数类型使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum LogLevel {
    /// 只显示错误信息。
    Error,
    /// 显示警告和错误信息（默认级别）。
    #[default]
    Warn,
    /// 显示处理进度与统计信息。
    Info,
    /// 显示调试细节，用于排查问题。
    Debug,
    /// 显示所有追踪信息，最详细也最嘈杂。
    Trace,
}

impl LogLevel {
    /// 转换为 `log` 库的 [`log::LevelFilter`]。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::runtime::cli::log_level::LogLevel;
    /// use log::LevelFilter;
    ///
    /// assert_eq!(LogLevel::Error.to_level_filter(), LevelFilter::Error);
    /// assert_eq!(LogLevel::Info.to_level_filter(), LevelFilter::Info);
    /// ```
    pub fn to_level_filter(self) -> log::LevelFilter {
        match self {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}
