use polars::prelude::*;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("unsupported file extension '{0}'; expected xlsx, xls, csv, or parquet")]
    UnsupportedExtension(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Polars error: {0}")]
    Polars(#[from] PolarsError),
    #[error("XLSX error: {0}")]
    Xlsx(String),
    #[error("workbook has no sheets")]
    NoSheets,
    #[error("sheet '{0}' is empty")]
    EmptySheet(String),
}

/// Dispatch to the correct reader based on file extension.
pub fn read_file(path: &Path) -> Result<DataFrame, ReadError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "xlsx" | "xls" | "ods" => read_xlsx(path),
        "csv" | "tsv" => read_csv(path),
        "parquet" => read_parquet(path),
        other => Err(ReadError::UnsupportedExtension(other.to_string())),
    }
}

/// Read the first sheet of an XLSX/XLS/ODS file into a DataFrame.
///
/// All cells are read as strings at this stage; type inference happens in the
/// mapping step.  Empty cells become nulls.
pub fn read_xlsx(path: &Path) -> Result<DataFrame, ReadError> {
    use calamine::{open_workbook_auto, DataType, Reader};

    let mut wb = open_workbook_auto(path)
        .map_err(|e| ReadError::Xlsx(e.to_string()))?;

    let sheet_names = wb.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err(ReadError::NoSheets);
    }

    let range = wb
        .worksheet_range(&sheet_names[0])
        .map_err(|e| ReadError::Xlsx(e.to_string()))?;

    let mut rows = range.rows();

    let header_row = rows.next().ok_or_else(|| ReadError::EmptySheet(sheet_names[0].clone()))?;

    let headers: Vec<String> = header_row
        .iter()
        .map(|c| {
            let s = c.to_string();
            if s.trim().is_empty() { String::new() } else { s }
        })
        .collect();

    let n_cols = headers.len();
    let mut columns: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols];

    for row in rows {
        for col_idx in 0..n_cols {
            let val = row.get(col_idx).and_then(|c| {
                if c.is_empty() {
                    None
                } else {
                    Some(c.to_string())
                }
            });
            columns[col_idx].push(val);
        }
    }

    if columns.is_empty() || columns[0].is_empty() {
        return Err(ReadError::EmptySheet(sheet_names[0].clone()));
    }

    let cols: Vec<Column> = headers
        .iter()
        .zip(columns.iter())
        .map(|(name, vals)| Column::from(Series::new(name.as_str().into(), vals)))
        .collect();

    DataFrame::new(cols).map_err(ReadError::Polars)
}

/// Read a CSV/TSV file into a DataFrame using Polars schema inference.
pub fn read_csv(path: &Path) -> Result<DataFrame, ReadError> {
    let sep = if path.extension().and_then(|e| e.to_str()) == Some("tsv") {
        b'\t'
    } else {
        b','
    };

    CsvReadOptions::default()
        .with_infer_schema_length(Some(100))
        .with_has_header(true)
        .map_parse_options(|opts| opts.with_separator(sep))
        .try_into_reader_with_file_path(Some(path.to_path_buf()))
        .map_err(ReadError::Polars)?
        .finish()
        .map_err(ReadError::Polars)
}

/// Read a Parquet file into a DataFrame.
pub fn read_parquet(path: &Path) -> Result<DataFrame, ReadError> {
    let file = std::fs::File::open(path)?;
    ParquetReader::new(file).finish().map_err(ReadError::Polars)
}

// ---------------------------------------------------------------------------
// Preview helper
// ---------------------------------------------------------------------------

/// Serialize the first `n_rows` of a DataFrame as row-major Option<String> for
/// display in the frontend table preview.
pub fn dataframe_preview(df: &DataFrame, n_rows: usize) -> Vec<Vec<Option<String>>> {
    let rows = df.height().min(n_rows);
    (0..rows)
        .map(|i| {
            df.get_columns()
                .iter()
                .map(|col| {
                    let av = col.get(i).unwrap_or(AnyValue::Null);
                    match av {
                        AnyValue::Null => None,
                        other => Some(other.to_string()),
                    }
                })
                .collect()
        })
        .collect()
}
