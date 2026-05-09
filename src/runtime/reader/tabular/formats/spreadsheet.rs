//! 电子表格读取实现（XLS/XLSX）。
//!
//! 与 CSV 不同，电子表格导出通常存在：
//! - 首部说明行；
//! - 非第一行才是真正表头；
//! - 日期/时间单元格为 Excel 序列值。
//!
//! 本模块负责把上述差异归一化到统一的 `TabularData`，供映射层复用。

use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};
use log::{debug, info, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    model::mapping::field_mapping::FieldMapping,
    utils::time::format_excel_datetime_serial,
};

use crate::runtime::reader::tabular::{
    TabularRecordReader,
    table::{RowData, TabularData},
};

impl TabularRecordReader {
    /// 读取电子表格（XLS/XLSX）并转换为统一表格结构。
    ///
    /// # 参数
    /// - `path`：电子表格路径。
    /// - `mapping`：字段映射配置，用于辅助自动识别表头行。
    ///
    /// # 返回值
    /// 返回归一化后的内部 `TabularData`。
    ///
    /// # 错误
    /// 工作簿无法打开、工作表读取失败等配置/输入错误。
    pub(in crate::runtime::reader::tabular) fn read_spreadsheet_table(
        &self,
        path: &Path,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<TabularData> {
        info!("Detected spreadsheet input, using workbook reader");

        let mut workbook = open_workbook_auto(path).map_err(|error| {
            let hint = if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xls"))
                .unwrap_or(false)
            {
                "\n  Hint: this file may be an HTML export disguised as .xls. \
                 Try opening it in Excel/WPS and saving as .xlsx, \
                 or use the .xlsx version if available."
            } else {
                ""
            };
            ImporterError::Config(format!(
                "Failed to open spreadsheet file '{}': {}{}",
                path.display(),
                error,
                hint
            ))
        })?;

        let Some(sheet_name) = workbook.sheet_names().first().cloned() else {
            warn!("No worksheet found in spreadsheet file: {}", path.display());
            return Ok(TabularData {
                source_name: "XLSX",
                headers: Vec::new(),
                rows: Vec::new(),
                pre_parse_errors: 0,
            });
        };

        let range = workbook.worksheet_range(&sheet_name).map_err(|error| {
            ImporterError::Config(format!(
                "Failed to read worksheet '{}' from '{}': {}",
                sheet_name,
                path.display(),
                error
            ))
        })?;

        let raw_rows = range
            .rows()
            .map(|row| {
                row.iter()
                    .map(Self::normalize_spreadsheet_cell)
                    .collect::<Vec<String>>()
            })
            .skip(self.skip_lines)
            .collect::<Vec<Vec<String>>>();

        if raw_rows.is_empty() {
            warn!(
                "No data lines found in worksheet '{}' after skipping {} lines",
                sheet_name, self.skip_lines
            );
            return Ok(TabularData {
                source_name: "XLSX",
                headers: Vec::new(),
                rows: Vec::new(),
                pre_parse_errors: 0,
            });
        }

        let (headers, rows) = if self.has_header {
            // XLSX 常见“前几行是说明文字”的情况：根据映射配置自动选择最像表头的那一行。
            let header_row_offset = self.select_xlsx_header_row(&raw_rows, mapping);
            let headers = raw_rows
                .get(header_row_offset)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|header| header.trim().to_string())
                .collect::<Vec<_>>();

            info!(
                "XLSX headers (row {}, {} columns): {:?}",
                header_row_offset + self.skip_lines + 1,
                headers.len(),
                headers
            );

            let rows = raw_rows
                .into_iter()
                .skip(header_row_offset + 1)
                .enumerate()
                .map(|(index, cells)| RowData {
                    line_no: index + self.skip_lines + header_row_offset + 2,
                    cells,
                })
                .collect::<Vec<_>>();

            (headers, rows)
        } else {
            debug!("No header row in XLSX, generated positional headers");
            let headers = Self::build_positional_headers();
            let rows = raw_rows
                .into_iter()
                .enumerate()
                .map(|(index, cells)| RowData {
                    line_no: index + self.skip_lines + 1,
                    cells,
                })
                .collect::<Vec<_>>();
            (headers, rows)
        };

        Ok(TabularData {
            source_name: "XLSX",
            headers,
            rows,
            pre_parse_errors: 0,
        })
    }

    /// 在开启表头模式时，自动识别最可能的 XLSX 表头行。
    ///
    /// 若未提供映射配置，则默认第一行是表头。
    fn select_xlsx_header_row(
        &self,
        rows: &[Vec<String>],
        mapping: Option<&FieldMapping>,
    ) -> usize {
        let Some(mapping) = mapping else {
            return 0;
        };

        let (best_index, best_score) = rows
            .iter()
            .enumerate()
            .map(|(index, row)| (index, self.xlsx_header_match_score(mapping, row)))
            .max_by_key(|(_, score)| *score)
            .unwrap_or((0, 0));

        if best_score == 0 {
            warn!("Unable to auto-detect XLSX header row by mapping, fallback to first row");
        }

        best_index
    }

    /// 计算某一行作为 XLSX 表头时与映射配置的匹配分数。
    ///
    /// 计分规则：
    /// - 命中一个标准映射列 +1；
    /// - 命中一个 `extra_fields` 映射列 +1。
    pub(in crate::runtime::reader::tabular) fn xlsx_header_match_score(
        &self,
        mapping: &FieldMapping,
        row: &[String],
    ) -> usize {
        let normalized = row.iter().map(|value| value.trim()).collect::<Vec<_>>();

        let mut score = 0usize;

        for (_, spec) in Self::mapped_specs(mapping) {
            if let Some(spec) = spec
                && normalized
                    .iter()
                    .any(|header| *header == spec.column_name())
            {
                score += 1;
            }
        }

        for column in mapping.extra_fields.values() {
            if normalized.contains(&column.as_str()) {
                score += 1;
            }
        }

        score
    }

    /// 将科学计数法字符串中代表整数的值还原为完整数字。
    ///
    /// 仅对含 `E`/`e` 且解析后无小数部分的有限浮点数生效。
    /// 常规字符串不受影响。
    fn normalize_scientific_integer_string(s: &str) -> Option<String> {
        if !s.contains(['E', 'e']) {
            return None;
        }
        let value: f64 = s.parse().ok()?;
        if value.is_finite() && value.fract() == 0.0 {
            Some(format!("{:.0}", value))
        } else {
            None
        }
    }

    /// 规范化电子表格单元格文本。
    ///
    /// 对日期时间序列值会先转为可读字符串，避免后续映射阶段再感知底层类型。
    /// 对整数型浮点值（如 11 位以上的产品账号）会用精确整数格式写出，
    /// 避免默认 Display 产生的科学计数法导致精度丢失。
    fn normalize_spreadsheet_cell(cell: &Data) -> String {
        match cell {
            Data::DateTime(datetime) => format_excel_datetime_serial(datetime.as_f64()),
            Data::DateTimeIso(value) | Data::DurationIso(value) | Data::String(value) => {
                let trimmed = value.trim();
                // XLSX 中将大整数存为文本时可能以科学计数法写出
                // （如 "2.40599E+11"），此处还原为完整整数格式。
                if let Some(normalized) = Self::normalize_scientific_integer_string(trimmed) {
                    return normalized;
                }
                trimmed.to_string()
            }
            Data::Float(value) => {
                let value = *value;
                // 对无小数部分的有限浮点数按整数写出，避免“产品账号”等
                // 大整数被 calamine 的默认 Display 以科学计数法截断。
                if value.fract() == 0.0 && value.is_finite() {
                    format!("{:.0}", value)
                } else {
                    value.to_string()
                }
            }
            Data::Int(value) => value.to_string(),
            _ => cell.to_string().trim().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use calamine::Data;

    use super::TabularRecordReader;

    #[test]
    fn large_integer_float_formatted_without_scientific_notation() {
        // 模拟产品账号/订单号等 12 位整数值
        let cell = Data::Float(240599141221.0);
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "240599141221");

        // 更小的整数不受影响
        let cell = Data::Float(100.0);
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "100");
    }

    #[test]
    fn fraction_float_keeps_decimal() {
        let cell = Data::Float(3.14);
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "3.14");
    }

    #[test]
    fn string_scientific_notation_integer_normalized() {
        let cell = Data::String("2.40599E+11".to_string());
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "240599000000");
    }

    #[test]
    fn string_without_scientific_notation_passes_through() {
        let cell = Data::String("240599141221".to_string());
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "240599141221");
    }

    #[test]
    fn string_scientific_fraction_passes_through() {
        let cell = Data::String("1.5E+2".to_string());
        let result = TabularRecordReader::normalize_spreadsheet_cell(&cell);
        assert_eq!(result, "150");
    }
}
