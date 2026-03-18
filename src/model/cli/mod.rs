//! 模块说明：命令行参数模型与日志级别解析。
//!
//! 文件路径：src/model/cli/mod.rs。
//! 该文件主要承担子模块声明与导出职责。
//! 关键符号：args、log_level。

pub mod args;
pub mod log_level;

pub use args::Cli;
