//! 按文件格式组织的读取实现。
//!
//! 本模块只负责把具体格式实现拆分到子模块：
//! - [`csv`]：CSV 读取与基础规范化。
//! - [`spreadsheet`]：XLS/XLSX 读取与表头识别。
//!
//! 两者最终都输出统一的内部表格结构，供映射层复用。

pub(super) mod csv;
pub(super) mod spreadsheet;
