//! 【模块文档】
//!
//! 模块名称：源码模块
//! 文件路径：
//! 核心职责：承担当前文件对应的功能实现
//! 主要输入：上游模块传入的数据
//! 主要输出：下游模块消费的数据或行为
//! 维护说明：变更前应确认其在导入链路中的位置与影响
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
