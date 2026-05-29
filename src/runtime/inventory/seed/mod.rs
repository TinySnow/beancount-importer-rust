//! 库存 seed 子模块。
//!
//! 该子模块负责从外部 Beancount seed 文件回放历史持仓变化，
//! 为跨期 lot 匹配提供初始库存状态。
//!
//! - [`loader`]：遍历文件列表、尽力加载 seed 库存；
//! - [`parser`]：解析单个 seed 文件中的交易头与过账行。

pub(crate) mod loader;
pub(crate) mod parser;

pub(crate) use loader::load_seed_inventory_from_files;
