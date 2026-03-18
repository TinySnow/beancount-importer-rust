//! 模块说明：通用工具函数集合。
//!
//! 文件路径：src/utils/init.rs。
//! 该文件围绕 'init' 的职责提供实现。
//! 关键符号：init_logger。

use env_logger::Builder;
use log::LevelFilter;

/// 初始化日志系统
pub fn init_logger(level: LevelFilter) {
    Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            use std::io::Write;

            let level_style = match record.level() {
                log::Level::Error => "\x1b[1;31m", // 粗体红色
                log::Level::Warn => "\x1b[1;33m",  // 粗体黄色
                log::Level::Info => "\x1b[1;32m",  // 粗体绿色
                log::Level::Debug => "\x1b[36m",   // 青色
                log::Level::Trace => "\x1b[90m",   // 灰色
            };
            let reset = "\x1b[0m";

            // 在 `Debug` 和 `Trace` 级别显示更多信息
            if record.level() <= log::Level::Debug {
                writeln!(
                    buf,
                    "{}{:>5}{} [{}:{}] {}",
                    level_style,
                    record.level(),
                    reset,
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.args()
                )
            } else {
                writeln!(
                    buf,
                    "{}{:>5}{}: {}",
                    level_style,
                    record.level(),
                    reset,
                    record.args()
                )
            }
        })
        .init();
}
