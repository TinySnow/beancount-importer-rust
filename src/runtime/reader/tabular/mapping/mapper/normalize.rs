//! 单元格文本规范化工具。
//!
//! 处理 Excel 导出中常见的 `="literal"` 格式展开。

/// 规范化单元格文本。
///
/// 目前除了常规 `trim` 外，还会展开 Excel 的 `="literal"` 形式。
pub(crate) fn normalize_cell_value(value: &str) -> String {
    let trimmed = value.trim();
    strip_excel_quoted_literal(trimmed).unwrap_or_else(|| trimmed.to_string())
}

/// 尝试展开 Excel 导出中的 `="literal"` 字面量包裹格式。
///
/// 返回 `Some` 表示成功展开；`None` 表示输入不是该模式。
fn strip_excel_quoted_literal(value: &str) -> Option<String> {
    if !value.starts_with('=') {
        return None;
    }

    let expression = value[1..].trim();
    if expression.len() < 2 || !expression.starts_with('"') || !expression.ends_with('"') {
        return None;
    }

    let inner = &expression[1..expression.len() - 1];
    Some(inner.replace("\"\"", "\""))
}
