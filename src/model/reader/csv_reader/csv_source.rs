use std::{fs::File, path::Path};

use csv::ReaderBuilder;
use log::{debug, info, warn};

use crate::{
    error::{ImporterError, ImporterResult},
    utils::encoding::decode_file,
};

use super::{
    CsvRecordReader,
    table::{RowData, TabularData},
};

impl CsvRecordReader {
    /// Read CSV file and normalize to `TabularData`.
    pub(super) fn read_csv_table(&self, path: &Path) -> ImporterResult<TabularData> {
        info!("Opening file: {}", path.display());
        let file = File::open(path)?;
        let file_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        debug!("File size: {} bytes", file_size);

        let content = decode_file(file, &self.csv_options.encoding)?;
        debug!("Decoded content length: {} chars", content.len());

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
            .delimiter(self.csv_options.delimiter as u8)
            .quote(self.csv_options.quote as u8)
            .flexible(self.csv_options.flexible)
            .has_headers(self.has_header);

        if let Some(comment) = self.csv_options.comment {
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

                    if self.strict_mode || !self.csv_options.flexible {
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
