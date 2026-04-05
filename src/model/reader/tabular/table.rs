//! 表格读取的内部中间结构。
//!
//! `csv` 与 `spreadsheet` 两种格式读取器都会先把源数据转换为本模块定义的
//! `TabularData`，之后再统一进入字段映射流程。
//!
//! 该模块为 `tabular` 子系统内部实现细节，不对外暴露。

/// 无表头模式下预生成的位置列名数量上限。
pub(super) const POSITIONAL_HEADER_COUNT: usize = 256;

/// 单行表格数据。
#[derive(Debug)]
pub(super) struct RowData {
    /// 可读行号（从 1 开始），用于日志和错误定位。
    pub(super) line_no: usize,
    /// 当前行全部单元格文本（已做基本 trim）。
    pub(super) cells: Vec<String>,
}

/// 统一后的表格数据模型。
///
/// 无论数据来源是 CSV 还是电子表格，最终都会转换为该结构后再执行映射。
#[derive(Debug)]
pub(super) struct TabularData {
    /// 源类型名称（如 `CSV`、`XLSX`），用于日志标识。
    pub(super) source_name: &'static str,
    /// 归一化后的表头。
    pub(super) headers: Vec<String>,
    /// 数据行。
    pub(super) rows: Vec<RowData>,
    /// 在进入映射流程之前已经统计到的解析错误数。
    pub(super) pre_parse_errors: usize,
}

/// 构造无表头模式下的默认位置列名（`col_0..col_255`）。
pub(super) fn build_positional_headers() -> Vec<String> {
    (0..POSITIONAL_HEADER_COUNT)
        .map(|index| format!("col_{}", index))
        .collect()
}
