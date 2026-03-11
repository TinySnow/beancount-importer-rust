use std::path::Path;

use calamine::{Reader, open_workbook_auto};
use log::{debug, info, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    model::mapping::field_mapping::FieldMapping,
};

use super::{
    CsvRecordReader,
    table::{RowData, TabularData},
};

impl CsvRecordReader {
    /// 读取 XLSX 并转换为统一表格结构。
    pub(super) fn read_xlsx_table(
        &self,
        path: &Path,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<TabularData> {
        info!("Detected XLSX input, using spreadsheet reader");

        let mut workbook = open_workbook_auto(path).map_err(|error| {
            ImporterError::Config(format!(
                "Failed to open XLSX file '{}': {}",
                path.display(),
                error
            ))
        })?;

        let Some(sheet_name) = workbook.sheet_names().first().cloned() else {
            warn!("No worksheet found in XLSX file: {}", path.display());
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
                    .map(|cell| cell.to_string())
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

    /// 在开启表头模式时，为 XLSX 自动识别最可能的表头行。
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
    pub(super) fn xlsx_header_match_score(&self, mapping: &FieldMapping, row: &[String]) -> usize {
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
            if normalized.iter().any(|header| *header == column.as_str()) {
                score += 1;
            }
        }

        score
    }
}
