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
/// 首次调用初始化全局 logger；后续调用通过 `set_max_level` 覆盖日志级别，不会 panic。
pub fn init_logger(level: LevelFilter) {
    let init_result = Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            use std::io::Write;

            let level_style = match record.level() {
                log::Level::Error => "\x1b[1;31m",
                log::Level::Warn => "\x1b[1;33m",
                log::Level::Info => "\x1b[1;32m",
                log::Level::Debug => "\x1b[36m",
                log::Level::Trace => "\x1b[90m",
            };
            let reset = "\x1b[0m";

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
        .try_init();
    // 若已初始化（如 batch 模式覆盖日志级别），通过 set_max_level 生效
    if init_result.is_err() {
        log::set_max_level(level);
    }
}
