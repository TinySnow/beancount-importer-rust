//! 表格行到 `RawRecord` 的映射编排。
//!
//! 该模块是 `tabular` 读取流程的"第二阶段"，负责把统一表格结构映射为
//! [`RawRecord`](crate::model::data::raw_record::RawRecord)。
//!
//! 子模块分工：
//! - [`record_mapper`]：单行字段映射、扩展字段与兜底推断；
//! - [`field_resolver`]：日期/数值/文本字段解析工具；
//! - [`normalize`]：单元格文本规范化。

use std::collections::HashMap;

use log::{info, trace, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    model::{
        data::raw_record::RawRecord,
        mapping::{field_mapping::FieldMapping},
    },
};

mod field_resolver;
mod normalize;
mod record_mapper;

pub(crate) use normalize::normalize_cell_value;

use crate::runtime::reader::tabular::{TabularRecordReader, table::TabularData};

impl TabularRecordReader {
    /// 将表格数据映射为标准 `RawRecord` 列表。
    ///
    /// # 参数
    /// - `table`：格式读取阶段产出的统一表格结构。
    /// - `mapping`：字段映射配置，可选。
    ///
    /// # 返回值
    /// 返回映射后的记录集合。
    ///
    /// # 错误
    /// 严格模式下遇到字段数量不一致或映射异常时返回错误。
    pub(in crate::runtime::reader::tabular) fn map_table_to_records(
        &self,
        table: TabularData,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<Vec<RawRecord>> {
        if let Some(mapping) = mapping {
            self.validate_mapping(mapping, &table.headers);
        }

        let expected_columns = table.headers.len();
        let mut records = Vec::new();
        let mut mapping_errors = 0usize;

        for mut row in table.rows {
            if Self::is_blank_row(&row.cells) || Self::is_summary_row(&row.cells) {
                continue;
            }

            // 某些银行 CSV 数据行会在末尾追加一个分隔符，导致"多 1 个空列"。
            // 这里裁掉超出的尾部空列，避免把格式噪音当成结构错误。
            while row.cells.len() > expected_columns
                && row
                    .cells
                    .last()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(false)
            {
                row.cells.pop();
            }

            if row.cells.len() != expected_columns {
                warn!(
                    "Line {}: field count mismatch (expected {}, got {})",
                    row.line_no,
                    expected_columns,
                    row.cells.len()
                );

                if self.strict_mode {
                    return Err(ImporterError::Parse {
                        line: row.line_no,
                        message: format!(
                            "Field count mismatch (expected {}, got {})",
                            expected_columns,
                            row.cells.len()
                        ),
                    });
                }
            }

            let field_map = table
                .headers
                .iter()
                .zip(row.cells.iter())
                .map(|(header, value)| (header.clone(), normalize_cell_value(value)))
                .collect::<HashMap<_, _>>();

            match self.map_to_raw_record(&field_map, mapping) {
                Ok(record) => records.push(record),
                Err(error) => {
                    mapping_errors += 1;
                    warn!("Line {}: mapping error - {}", row.line_no, error);

                    if self.strict_mode {
                        return Err(ImporterError::Parse {
                            line: row.line_no,
                            message: format!("Mapping error: {error}"),
                        });
                    }
                }
            }
        }

        let total_errors = table.pre_parse_errors + mapping_errors;
        info!(
            "{} parsing complete: {} records parsed, {} errors",
            table.source_name,
            records.len(),
            total_errors
        );

        Ok(records)
    }

    /// 是否为空白行。
    fn is_blank_row(cells: &[String]) -> bool {
        cells.iter().all(|value| value.trim().is_empty())
    }

    /// 是否为汇总/合计尾行。
    fn is_summary_row(cells: &[String]) -> bool {
        let Some(first_non_empty) = cells.iter().find_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }) else {
            return false;
        };

        first_non_empty.contains("合计")
            || first_non_empty.eq_ignore_ascii_case("total")
            || first_non_empty.eq_ignore_ascii_case("subtotal")
    }

    /// 校验 mapping 中引用的列名是否存在于表头。
    fn validate_mapping(&self, mapping: &FieldMapping, headers: &[String]) {
        for (name, spec) in Self::mapped_specs(mapping) {
            if let Some(spec) = spec {
                let column = spec.column_name();
                if headers.iter().any(|header| header == column) {
                    trace!("Mapping '{}' -> '{}'", name, column);
                } else {
                    warn!(
                        "Mapping field '{}' references column '{}' that is not in source headers",
                        name, column
                    );
                }
            }
        }
    }
}
