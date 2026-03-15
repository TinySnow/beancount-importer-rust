//! CSV/XLSX statement reader and field mapping entry.

use std::path::Path;

use crate::{
    error::ImporterResult,
    model::{
        config::csv_options::CsvOptions,
        data::raw_record::RawRecord,
        mapping::{field_mapping::FieldMapping, field_spec::FieldSpec},
    },
};

mod csv_source;
mod mapper;
mod table;
mod xlsx_source;

use table::build_positional_headers;

/// Reader that loads CSV/XLSX rows and maps them to `RawRecord`.
pub struct CsvRecordReader {
    csv_options: CsvOptions,
    skip_lines: usize,
    has_header: bool,
    strict_mode: bool,
}

impl CsvRecordReader {
    /// Build a new reader.
    pub fn new(
        csv_options: CsvOptions,
        skip_lines: usize,
        has_header: bool,
        strict_mode: bool,
    ) -> Self {
        Self {
            csv_options,
            skip_lines,
            has_header,
            strict_mode,
        }
    }

    /// Read a source file and map rows into `RawRecord`.
    pub fn read_file(
        &self,
        path: &Path,
        mapping: Option<&FieldMapping>,
    ) -> ImporterResult<Vec<RawRecord>> {
        let table = if Self::is_xlsx_path(path) {
            self.read_xlsx_table(path, mapping)?
        } else {
            self.read_csv_table(path)?
        };

        self.map_table_to_records(table, mapping)
    }

    /// Return true if the path has `.xlsx` extension.
    fn is_xlsx_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("xlsx"))
            .unwrap_or(false)
    }

    /// Build synthetic headers for no-header mode.
    fn build_positional_headers() -> Vec<String> {
        build_positional_headers()
    }

    /// Return all standard mapping specs used for validation/scoring.
    fn mapped_specs<'a>(mapping: &'a FieldMapping) -> [(&'static str, Option<&'a FieldSpec>); 14] {
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
    use crate::model::{
        config::csv_options::CsvOptions,
        mapping::{field_mapping::FieldMapping, field_spec::FieldSpec},
    };

    use super::CsvRecordReader;

    #[test]
    fn xlsx_header_score_prefers_real_header_row() {
        let reader = CsvRecordReader::new(CsvOptions::default(), 0, true, false);

        let mut mapping = FieldMapping::default();
        mapping.date = Some(FieldSpec::Simple("date".to_string()));
        mapping.amount = Some(FieldSpec::Simple("amount".to_string()));
        mapping.payee = Some(FieldSpec::Simple("payee".to_string()));

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
        let headers = CsvRecordReader::build_positional_headers();
        assert_eq!(headers.len(), 256);
        assert_eq!(headers[0], "col_0");
        assert_eq!(headers[255], "col_255");
    }
}
