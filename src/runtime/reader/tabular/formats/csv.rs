//! CSV 读取实现。
//!
//! 本模块负责：
//! - 根据 [`TabularOptions`](crate::model::config::tabular_options::TabularOptions)
//!   构造 `csv` 解析器；
//! - 对输入文本做基础清洗（跳行、trim）；
//! - 产出统一的内部 `TabularData` 结构，供映射层复用。

use std::{fs::File, path::Path};

use csv::ReaderBuilder;
use log::{debug, info, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    utils::encoding::decode_file,
};

use crate::runtime::reader::tabular::{
    TabularRecordReader,
    table::{RowData, TabularData},
};

impl TabularRecordReader {
    /// 读取 CSV 文件并归一化为内部表格结构。
    ///
    /// # 参数
    /// - `path`：CSV 文件路径。
    ///
    /// # 返回值
    /// 返回解析后的 [`TabularData`](crate::runtime::reader::tabular::table::TabularData)。
    ///
    /// # 错误
    /// - 文件打开或解码失败；
    /// - CSV 语法错误且当前策略要求立即失败（严格模式或禁用 flexible）。
    pub(in crate::runtime::reader::tabular) fn read_csv_table(
        &self,
        path: &Path,
    ) -> ImporterResult<TabularData> {
        info!("Opening file: {}", path.display());
        let file = File::open(path)?;
        let file_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        debug!("File size: {} bytes", file_size);

        let content = decode_file(file, &self.tabular_options.encoding)?;
        debug!("Decoded content length: {} chars", content.len());

        // `skip_lines` 在文本层先执行，后续 csv 解析器只看到有效数据区域。
        let lines: Vec<&str> = content.lines().skip(self.skip_lines).collect();
        if lines.is_empty() {
            warn!(
                "No data lines found after skipping {} lines",
                self.skip_lines
            );
            return Ok(TabularData {
                source_name: "CSV",
                headers: Vec::new(),
                rows: Vec::new(),
                pre_parse_errors: 0,
            });
        }

        let content = lines.join("\n");

        let mut builder = ReaderBuilder::new();
        builder
            .delimiter(self.tabular_options.delimiter as u8)
            .quote(self.tabular_options.quote as u8)
            .flexible(self.tabular_options.flexible)
            .has_headers(self.has_header);

        if let Some(comment) = self.tabular_options.comment {
            builder.comment(Some(comment as u8));
        }

        let mut csv_reader = builder.from_reader(content.as_bytes());

        let headers = if self.has_header {
            let parsed_headers = csv_reader
                .headers()?
                .iter()
                .map(|header| header.trim().to_string())
                .collect::<Vec<_>>();

            info!(
                "CSV headers ({} columns): {:?}",
                parsed_headers.len(),
                parsed_headers
            );

            parsed_headers
        } else {
            debug!("No header row, generated positional headers");
            Self::build_positional_headers()
        };

        let mut rows = Vec::new();
        let mut pre_parse_errors = 0usize;

        for (line_index, row_result) in csv_reader.records().enumerate() {
            // `line_index` 是从“数据起始行”计数，需还原为原文件可读行号。
            let actual_line = line_index + self.skip_lines + if self.has_header { 2 } else { 1 };

            match row_result {
                Ok(row) => {
                    rows.push(RowData {
                        line_no: actual_line,
                        cells: row.iter().map(|value| value.trim().to_string()).collect(),
                    });
                }
                Err(error) => {
                    pre_parse_errors += 1;
                    warn!("Line {}: CSV parse error - {}", actual_line, error);

                    // 严格模式下必须立刻失败；非严格模式仅在 flexible=true 时允许跳过坏行。
                    if self.strict_mode || !self.tabular_options.flexible {
                        return Err(ImporterError::Parse {
                            line: actual_line,
                            message: error.to_string(),
                        });
                    }
                }
            }
        }

        Ok(TabularData {
            source_name: "CSV",
            headers,
            rows,
            pre_parse_errors,
        })
    }
}
