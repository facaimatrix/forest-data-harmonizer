use polars::prelude::*;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("unsupported file extension '{0}'; expected xlsx, xls, csv, tsv, txt, or parquet")]
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
/// Empty / spacer columns are dropped; blank headers are renamed.
pub fn read_file(path: &Path) -> Result<DataFrame, ReadError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let df = match ext.as_str() {
        "xlsx" | "xls" | "ods" => read_xlsx(path)?,
        "csv" | "tsv" | "txt" => read_csv(path)?,
        "parquet" => read_parquet(path)?,
        other => return Err(ReadError::UnsupportedExtension(other.to_string())),
    };

    Ok(normalize_dataframe(df))
}

/// Drop all-null columns and fix blank / duplicate header names.
pub fn normalize_dataframe(df: DataFrame) -> DataFrame {
    let height = df.height();
    if height == 0 {
        return df;
    }

    let non_empty: Vec<Column> = df
        .get_columns()
        .iter()
        .filter(|s| s.null_count() < height)
        .cloned()
        .map(Column::from)
        .collect();

    if non_empty.is_empty() {
        return df;
    }

    let mut df = DataFrame::new(non_empty).unwrap_or(df);

    let old_names: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let new_names: Vec<String> = old_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let trimmed = name.trim();
            let base = if trimmed.is_empty() || name.starts_with("column_") {
                format!("_unnamed_{}", i + 1)
            } else {
                trimmed.to_string()
            };
            let count = seen.entry(base.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{}_{}", base, count)
            }
        })
        .collect();

    for (old, new) in old_names.iter().zip(new_names.iter()) {
        if old != new {
            let _ = df.rename(old, new.as_str().into());
        }
    }

    df
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

/// Read a delimited text file (CSV / TSV / TXT) into a DataFrame.
///
/// All columns are read as strings (`infer_schema_length = 0`), matching the
/// XLSX path. Polars' default 100-row inference otherwise treats early numeric
/// Status codes as `i64` and fails when later rows contain labels like `"snag"`.
/// Semantic casting happens in the mapping / transform steps.
///
/// Separator: `.tsv` → tab; `.csv` → comma; `.txt` (and others) → sniffed from
/// the first non-empty line (comma, tab, or semicolon — whichever is most common).
pub fn read_csv(path: &Path) -> Result<DataFrame, ReadError> {
    let sep = detect_separator(path)?;

    CsvReadOptions::default()
        .with_infer_schema_length(Some(0))
        .with_has_header(true)
        .map_parse_options(|opts| opts.with_separator(sep))
        .try_into_reader_with_file_path(Some(path.to_path_buf()))
        .map_err(ReadError::Polars)?
        .finish()
        .map_err(ReadError::Polars)
}

fn detect_separator(path: &Path) -> Result<u8, ReadError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "tsv" => return Ok(b'\t'),
        "csv" => return Ok(b','),
        _ => {}
    }

    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).map_err(ReadError::Io)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(ReadError::Io)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let commas = trimmed.bytes().filter(|&b| b == b',').count();
            let tabs = trimmed.bytes().filter(|&b| b == b'\t').count();
            let semis = trimmed.bytes().filter(|&b| b == b';').count();
            return Ok(if tabs >= commas && tabs >= semis && tabs > 0 {
                b'\t'
            } else if semis > commas && semis >= tabs {
                b';'
            } else {
                b','
            });
        }
    }
    Ok(b',')
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
                        other => {
                            let s = other.str_value();
                            if s == "null" {
                                None
                            } else {
                                Some(s.into_owned())
                            }
                        }
                    }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn normalize_drops_all_null_and_blank_headers() {
        let df = DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1", "P2"])),
            Column::from(Series::new("".into(), &[None::<String>, None])),
            Column::from(Series::new("DBH".into(), &[Some(10.0), Some(12.0)])),
        ])
        .unwrap();
        let out = normalize_dataframe(df);
        assert_eq!(out.width(), 2);
        assert!(out.get_column_names().iter().any(|n| n.as_str() == "PlotID"));
        assert!(out.get_column_names().iter().any(|n| n.as_str() == "DBH"));
    }

    #[test]
    fn csv_reads_mixed_status_labels_as_string() {
        let dir = std::env::temp_dir();
        let path = dir.join("gfb3_status_snag_test.csv");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(
                f,
                "PlotID,TreeID,YR,DBH,Status\nP1,T1,2010,12.5,0\nP1,T2,2010,8.1,1\nP1,T3,2010,,snag"
            )
            .unwrap();
        }
        let df = read_csv(&path).expect("csv with late string Status should load");
        let _ = std::fs::remove_file(&path);
        assert_eq!(df.height(), 3);
        let status = df.column("Status").unwrap();
        assert_eq!(status.dtype(), &DataType::String);
        assert_eq!(status.str().unwrap().get(2), Some("snag"));
    }

    #[test]
    fn txt_comma_separated_loads_via_read_file() {
        let path = std::env::temp_dir().join("gfb3_txt_comma_test.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "PlotID,TreeID,YR\nP1,T1,2010\nP1,T2,2011").unwrap();
        }
        let df = read_file(&path).expect(".txt comma-separated should load");
        let _ = std::fs::remove_file(&path);
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 3);
        assert!(df.column("PlotID").is_ok());
    }

    #[test]
    fn txt_tab_separated_loads_via_read_file() {
        let path = std::env::temp_dir().join("gfb3_txt_tab_test.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "PlotID\tTreeID\tYR\nP1\tT1\t2010").unwrap();
        }
        let df = read_file(&path).expect(".txt tab-separated should load");
        let _ = std::fs::remove_file(&path);
        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 3);
    }
}
