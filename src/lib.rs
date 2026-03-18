//! 模块说明：库入口，负责导出可复用的核心模块。
//!
//! 文件路径：src/lib.rs。
//! 该文件提供库级入口与公共导出。
//! 关键符号：error、interface、model、providers。

pub mod error;
pub mod interface;
pub mod model;
pub mod providers;
pub mod utils;

mod runtime;

use anyhow::Result;

use crate::model::cli::Cli;

/// 运行导入主流程。
///
/// 该函数是二进制入口与运行时实现之间的薄封装，
/// 方便在测试中直接调用并保持职责清晰。
pub fn app(cli: Cli) -> Result<()> {
    runtime::run(cli)
}
