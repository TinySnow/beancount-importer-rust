//! 主入口模块
//!
//! 该模块是程序的命令行入口点，负责解析命令行参数并启动导入运行时。
//!
//! # 主要功能
//! - 解析命令行参数
//! - 初始化日志系统
//! - 调用核心应用逻辑
//! - 处理错误并设置退出码
//!
//! # 示例
//! ```bash
//! # 基本使用
//! beancount-importer-rust --provider alipay --source statement.csv
//!
//! # 启用详细日志
//! beancount-importer-rust --provider alipay --source statement.csv --verbose
//! ```
//!

use std::process;

use beancount_importer_rust::{app, runtime::cli::Cli, utils::init::init_logger};
use clap::Parser;
use log::{debug, info};

/// 可执行程序入口
///
/// 程序的主入口点，负责以下工作：
/// 1. 解析命令行参数，构建 `Cli` 对象
/// 2. 根据用户指定的日志级别初始化全局日志器
/// 3. 输出启动上下文信息，便于问题排查
/// 4. 调用核心应用逻辑 `app()` 函数
/// 5. 处理可能的错误并设置相应的退出码
///
/// # 错误处理
/// - 如果核心应用逻辑执行失败，会打印错误信息并以非零退出码退出
/// - 这样设计便于脚本和持续集成系统感知执行失败
///
/// # 日志输出
/// - 启动时会输出当前工作目录信息
/// - 在调试模式下会输出完整的命令行参数信息
fn main() {
    // 解析命令行参数，构建 Cli 对象
    let cli = Cli::parse();

    // 初始化日志系统，使用用户指定的日志级别
    init_logger(cli.effective_log_level());

    // 输出启动上下文信息，便于问题排查
    info!("Working directory: {:?}", std::env::current_dir());
    debug!("CLI args: {:?}", cli);

    // 调用核心应用逻辑并处理错误
    if let Err(err) = app(cli) {
        // 打印错误信息到标准错误输出
        eprintln!("Error: {err}");
        // 以非零退出码退出，指示执行失败
        process::exit(1);
    }
    // 执行成功，默认以零退出码退出
}
