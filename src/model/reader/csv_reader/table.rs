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
