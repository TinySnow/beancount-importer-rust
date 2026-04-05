//! 表格读取入口（CSV/XLS/XLSX）。
//!
//! 该模块把“格式解析 + 字段映射”封装到 [`TabularRecordReader`] 中，
//! 对外提供统一的文件读取接口。
//!
//! 读取流程分为两步：
//! 1. 根据扩展名选择 CSV 或电子表格读取实现，得到中间表格结构。
//! 2. 根据可选 [`FieldMapping`](crate::model::mapping::field_mapping::FieldMapping)
//!    将行数据映射为 [`RawRecord`](crate::model::data::raw_record::RawRecord)。
//!
//! # 示例
//! ```rust,no_run
//! use std::path::Path;
//!
//! use beancount_importer_rust::model::{
//!     config::tabular_options::TabularOptions,
//!     reader::tabular::TabularRecordReader,
//! };
//!
//! let reader = TabularRecordReader::new(
//!     TabularOptions::default(),
//!     1,    // 跳过首行说明文字
//!     true, // 源文件包含表头
//!     false,
//! );
//!
//! let _records = reader.read_file(Path::new("statement.xlsx"), None)?;
//! # Ok::<(), beancount_importer_rust::error::ImporterError>(())
//! ```

use std::path::Path;

use crate::{
    error::ImporterResult,
    model::{
        config::tabular_options::TabularOptions,
        data::raw_record::RawRecord,
        mapping::{field_mapping::FieldMapping, field_spec::FieldSpec},
    },
};

mod formats;
mod mapping;
mod table;

use table::build_positional_headers;

/// 读取 CSV/电子表格并映射为 `RawRecord` 的统一读取器。
pub struct TabularRecordReader {
    /// CSV/XLS(X) 通用解析配置（分隔符、编码、注释符等）。
    tabular_options: TabularOptions,
    /// 读取前跳过的原始行数。
    skip_lines: usize,
    /// 输入数据是否包含表头行。
    has_header: bool,
    /// 严格模式：遇到字段数量或映射错误时立即返回错误。
    strict_mode: bool,
}

impl TabularRecordReader {
    /// 创建表格读取器。
    ///
    /// # 参数
    /// - `tabular_options`：CSV/XLS(X) 通用解析参数。
    /// - `skip_lines`：读取前要跳过的行数。
    /// - `has_header`：是否包含表头。
    /// - `strict_mode`：是否在解析异常时立即失败。
    ///
    /// # 返回值
    /// 返回新的 [`TabularRecordReader`] 实例。
    ///
    /// # 示例
    /// ```rust
    /// use beancount_importer_rust::model::{
    ///     config::tabular_options::TabularOptions,
    ///     reader::tabular::TabularRecordReader,
    /// };
    ///
    /// let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);
    /// let _ = reader;
    /// ```
    pub fn new(
        tabular_options: TabularOptions,
        skip_lines: usize,
        has_header: bool,
        strict_mode: bool,
    ) -> Self {
        Self {
            tabular_options,
            skip_lines,
            has_header,
            strict_mode,
        }
    }

    /// 读取输入文件并映射为 [`RawRecord`](crate::model::data::raw_record::RawRecord) 列表。
    ///
    /// 当 `mapping` 为 `None` 时，读取器不会尝试映射标准字段，
    /// 而是把非空列作为 `extra` 字段写入原始记录。
    ///
    /// # 参数
    /// - `path`：输入文件路径。
    /// - `mapping`：字段映射配置，可为空。
    ///
    /// # 返回值
    /// 返回解析后的原始记录列表。
    ///
    /// # 错误
    /// 当文件读取失败、格式解析失败或严格模式下映射失败时返回错误。
    ///
    /// # 示例
    /// ```rust,no_run
    /// use std::path::Path;
    ///
    /// use beancount_importer_rust::model::{
    ///     config::tabular_options::TabularOptions,
    ///     reader::tabular::TabularRecordReader,
    /// };
    ///
    /// let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, true);
    /// let records = reader.read_file(Path::new("statement.csv"), None)?;
    /// assert!(records.is_empty() || !records.is_empty());
    /// # Ok::<(), beancount_importer_rust::error::ImporterError>(())
    /// ```
    pub fn read_file(
        &self,
        path: &Path,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<Vec<RawRecord>> {
        // 统一入口先做“格式路由”，后续都走同一套映射逻辑。
        let table = if Self::is_spreadsheet_path(path) {
            self.read_spreadsheet_table(path, mapping)?
        } else {
            self.read_csv_table(path)?
        };

        self.map_table_to_records(table, mapping)
    }

    /// 判断路径是否为电子表格扩展名。
    ///
    /// 当前支持：`xls`、`xlsx`、`xlsm`、`xlsb`。
    fn is_spreadsheet_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                ext.eq_ignore_ascii_case("xlsx")
                    || ext.eq_ignore_ascii_case("xls")
                    || ext.eq_ignore_ascii_case("xlsm")
                    || ext.eq_ignore_ascii_case("xlsb")
            })
            .unwrap_or(false)
    }

    /// 在“无表头模式”下构建位置列名（`col_0`、`col_1` ...）。
    fn build_positional_headers() -> Vec<String> {
        build_positional_headers()
    }

    /// 返回用于映射校验与表头打分的标准字段集合。
    ///
    /// 该函数返回固定顺序数组，避免迭代 `HashMap` 带来的顺序不确定性。
    fn mapped_specs(mapping: &FieldMapping) -> [(&'static str, Option<&FieldSpec>); 14] {
        [
            ("date", mapping.date.as_ref()),
            ("amount", mapping.amount.as_ref()),
            ("currency", mapping.currency.as_ref()),
            ("payee", mapping.payee.as_ref()),
            ("narration", mapping.narration.as_ref()),
            ("transaction_type", mapping.transaction_type.as_ref()),
            ("status", mapping.status.as_ref()),
            ("reference", mapping.reference.as_ref()),
            ("symbol", mapping.symbol.as_ref()),
            ("security_name", mapping.security_name.as_ref()),
            ("quantity", mapping.quantity.as_ref()),
            ("unit_price", mapping.unit_price.as_ref()),
            ("fee", mapping.fee.as_ref()),
            ("tax", mapping.tax.as_ref()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::model::{
        config::tabular_options::TabularOptions,
        mapping::{field_mapping::FieldMapping, field_spec::FieldSpec},
    };

    use super::TabularRecordReader;

    #[test]
    fn xlsx_header_score_prefers_real_header_row() {
        let reader = TabularRecordReader::new(TabularOptions::default(), 0, true, false);

        let mapping = FieldMapping {
            date: Some(FieldSpec::Simple("date".to_string())),
            amount: Some(FieldSpec::Simple("amount".to_string())),
            payee: Some(FieldSpec::Simple("payee".to_string())),
            ..FieldMapping::default()
        };

        let meta_row = vec!["meta title".to_string(), "".to_string()];
        let header_row = vec![
            "date".to_string(),
            "payee".to_string(),
            "amount".to_string(),
        ];

        let meta_score = reader.xlsx_header_match_score(&mapping, &meta_row);
        let header_score = reader.xlsx_header_match_score(&mapping, &header_row);

        assert!(header_score > meta_score);
    }

    #[test]
    fn positional_headers_have_fixed_size() {
        let headers = TabularRecordReader::build_positional_headers();
        assert_eq!(headers.len(), 256);
        assert_eq!(headers[0], "col_0");
        assert_eq!(headers[255], "col_255");
    }

    #[test]
    fn spreadsheet_path_detection_supports_xls_and_xlsx() {
        assert!(TabularRecordReader::is_spreadsheet_path(Path::new(
            "statement.xlsx"
        )));
        assert!(TabularRecordReader::is_spreadsheet_path(Path::new(
            "statement.xls"
        )));
        assert!(TabularRecordReader::is_spreadsheet_path(Path::new(
            "statement.xlsm"
        )));
        assert!(!TabularRecordReader::is_spreadsheet_path(Path::new(
            "statement.csv"
        )));
    }
}
