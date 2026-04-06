//! 库入口模块
//!
//! 该模块是库的主入口点，负责导出可复用的核心模块和功能。
//!
//! # 导出模块
//! - [`error`]：错误处理相关功能
//! - [`interface`]：接口定义
//! - [`model`]：数据模型定义
//! - [`providers`]：数据提供器实现
//! - [`utils`]：工具函数和辅助功能
//! - [`runtime`]：运行时读取/写出与供应商注册能力
//!
//! # 核心功能
//! - 提供 `app()` 函数作为二进制入口与运行时实现之间的薄封装
//! - 统一导出库的公共 API
//!
//! # 使用示例
//! ```rust,no_run
//! use beancount_importer_rust::{app, runtime::cli::Cli};
//! use clap::Parser;
//!
//! // 解析命令行参数
//! let cli = Cli::parse();
//!
//! // 运行导入主流程
//! if let Err(err) = app(cli) {
//!     eprintln!("Error: {err:#}");
//!     std::process::exit(1);
//! }
//! ```
//!

pub mod error;
pub mod interface;
pub mod model;
pub mod providers;
pub mod utils;

pub mod runtime;

use anyhow::Result;

use crate::runtime::cli::Cli;

/// 运行导入主流程
///
/// 该函数是二进制入口与运行时实现之间的薄封装，
/// 方便在测试中直接调用并保持职责清晰。
///
/// # 参数
/// - `cli`：命令行参数对象，包含配置信息和运行选项
///
/// # 返回值
/// - `Ok(())`：导入流程成功完成
/// - `Err(Error)`：导入流程失败，包含详细错误信息
///
/// # 示例
/// ```rust,no_run
/// use beancount_importer_rust::{app, runtime::cli::Cli};
/// use clap::Parser;
///
/// // 解析命令行参数
/// let cli = Cli::parse();
///
/// // 运行导入主流程
/// match app(cli) {
///     Ok(_) => println!("Import completed successfully"),
///     Err(err) => eprintln!("Import failed: {err}"),
/// }
/// ```
pub fn app(cli: Cli) -> Result<()> {
    // 调用内部运行时模块的 run 函数执行实际导入流程
    runtime::run(cli)
}
