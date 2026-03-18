//! 模块说明：命令行参数模型与日志级别解析。
//!
//! 文件路径：src/model/cli/log_level.rs。
//! 该文件围绕 'log_level' 的职责提供实现。
//! 关键符号：LogLevel、to_level_filter。

use clap::ValueEnum;

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum LogLevel {
    /// 只显示错误
    Error,
    /// 显示警告和错误
    #[default]
    Warn,
    /// 显示处理进度和统计信息
    Info,
    /// 显示详细调试信息
    Debug,
    /// 显示所有追踪信息
    Trace,
}

impl LogLevel {
    /// 转换为 `log` 库的 `LevelFilter`
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
