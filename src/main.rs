//! 模块说明：命令行入口，负责解析参数并启动导入运行时。
//!
//! 文件路径：src/main.rs。
//! 该文件提供可执行程序入口。
//! 关键符号：main。

use std::process;

use beancount_importer_rust::{app, model::cli::Cli, utils::init::init_logger};
use clap::Parser;
use log::{debug, info};

/// 可执行程序入口。
///
/// 这里仅负责三件事：
/// 1. 解析命令行参数；
/// 2. 初始化日志；
/// 3. 调用库入口并处理错误退出码。
fn main() {
    // 先解析命令行参数，后续所有运行行为都依赖该参数对象。
    let cli = Cli::parse();
    // 按用户指定的日志级别初始化全局日志器。
    init_logger(cli.effective_log_level());

    // 输出少量启动上下文，便于问题排查。
    info!("Working directory: {:?}", std::env::current_dir());
    debug!("CLI args: {:?}", cli);

    // 主流程失败时返回非 0 退出码，便于脚本与持续集成系统感知失败。
    if let Err(err) = app(cli) {
        eprintln!("Error: {err:#}");
        process::exit(1);
    }
}
