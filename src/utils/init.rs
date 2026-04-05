//! 应用初始化工具。
//!
//! 当前主要提供日志系统初始化能力。
//!
//! # 示例
//! ```rust,no_run
//! use beancount_importer_rust::utils::init::init_logger;
//! use log::LevelFilter;
//!
//! init_logger(LevelFilter::Info);
//! log::info!("logger initialized");
//! ```

use env_logger::Builder;
use log::LevelFilter;

/// 初始化全局日志系统。
///
/// 日志输出带 ANSI 颜色，并在 `Debug`/`Trace` 级别附带源码位置信息。
///
/// # 参数
/// - `level`：全局日志级别过滤器。
///
/// # 注意
/// 该函数底层调用 `env_logger` 全局初始化，同一进程通常只能成功初始化一次。
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

            // 在 `Debug` 和 `Trace` 级别显示更多上下文，便于定位问题。
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
