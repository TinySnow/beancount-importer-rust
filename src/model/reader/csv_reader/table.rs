//! 模块说明：CSV/XLS 源读取与字段映射解析能力。
//!
//! 文件路径：src/model/reader/csv_reader/table.rs。
//! 该文件围绕 'table' 的职责提供实现。
//! 关键符号：无显式公开符号，主要通过内部实现或模块组织发挥作用。

pub(super) const POSITIONAL_HEADER_COUNT: usize = 256;

/// 单行表格数据（附带源文件中的可读行号）。
#[derive(Debug)]
pub(super) struct RowData {
    pub(super) line_no: usize,
    pub(super) cells: Vec<String>,
}

/// 统一后的表格数据模型，CSV/XLSX 最终都会转成该结构再做字段映射。
#[derive(Debug)]
pub(super) struct TabularData {
    pub(super) source_name: &'static str,
    pub(super) headers: Vec<String>,
    pub(super) rows: Vec<RowData>,
    pub(super) pre_parse_errors: usize,
}

/// 构造无表头模式下的默认位置列名。
pub(super) fn build_positional_headers() -> Vec<String> {
    (0..POSITIONAL_HEADER_COUNT)
        .map(|index| format!("col_{}", index))
        .collect()
}
