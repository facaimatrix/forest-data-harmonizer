use chrono::Utc;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SCHEMA_VERSION: &str = "gfb3-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildKind {
    Draft,
    Authoritative,
}

impl BuildKind {
    pub fn stamp(&self) -> &'static str {
        match self {
            BuildKind::Draft => "draft",
            BuildKind::Authoritative => "authoritative",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub build: BuildKind,
    pub app_version: String,
    pub schema_version: String,
    pub in_dsn: String,
    pub timestamp: String,
}

impl Provenance {
    pub fn new_draft(in_dsn: &str) -> Self {
        Provenance {
            build: BuildKind::Draft,
            app_version: APP_VERSION.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            in_dsn: in_dsn.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn csv_header_comment(&self) -> String {
        format!(
            "# build={} app_version={} schema_version={} in_dsn={} timestamp={}",
            self.build.stamp(),
            self.app_version,
            self.schema_version,
            self.in_dsn,
            self.timestamp
        )
    }
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Polars error during export: {0}")]
    Polars(#[from] PolarsError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XLSX error: {0}")]
    Xlsx(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Coerce the Status column from String to Int32.
/// Must be called last in the export pipeline — Status is String throughout.
pub fn coerce_status_to_int(lf: LazyFrame) -> LazyFrame {
    lf.with_columns([col("Status").cast(DataType::Int32)])
}

/// Build the output filename with the appropriate draft suffix.
pub fn draft_filename(base_name: &str, ext: &str) -> String {
    format!("{}_DRAFT.{}", base_name, ext)
}

/// Draft filename for paired-census GFB3 exports.
pub fn gfb3_draft_filename(base_name: &str, ext: &str) -> String {
    format!("{}_GFB3_DRAFT.{}", base_name, ext)
}

/// Draft filename for the plot-level summary table.
pub fn plots_summary_filename(base_name: &str, ext: &str) -> String {
    format!("{}_plots_summary_DRAFT.{}", base_name, ext)
}

/// Draft filename for the dataset-level summary table.
pub fn dataset_summary_filename(base_name: &str, ext: &str) -> String {
    format!("{}_dataset_summary_DRAFT.{}", base_name, ext)
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

/// Write a DataFrame to CSV with the header row first (no preamble).
///
/// Provenance is written as a sidecar JSON at `<stem>.provenance.json`, matching
/// the Parquet export convention — CSV bodies stay import-safe for Excel/R/Python.
pub fn write_csv(mut df: DataFrame, path: &Path, provenance: &Provenance) -> Result<(), ExportError> {
    let file = std::fs::File::create(path)?;
    CsvWriter::new(file)
        .finish(&mut df)
        .map_err(ExportError::Polars)?;
    write_provenance_sidecar(path, provenance)
}

/// Write a DataFrame to CSV with no provenance sidecar (header row first).
pub fn write_csv_plain(mut df: DataFrame, path: &Path) -> Result<(), ExportError> {
    let file = std::fs::File::create(path)?;
    CsvWriter::new(file)
        .finish(&mut df)
        .map_err(ExportError::Polars)
}

fn write_provenance_sidecar(path: &Path, provenance: &Provenance) -> Result<(), ExportError> {
    let sidecar = path.with_extension("provenance.json");
    let json = serde_json::to_string_pretty(provenance)?;
    std::fs::write(sidecar, json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Parquet
// ---------------------------------------------------------------------------

/// Write a DataFrame to Parquet with no provenance sidecar.
pub fn write_parquet_plain(mut df: DataFrame, path: &Path) -> Result<(), ExportError> {
    let file = std::fs::File::create(path)?;
    ParquetWriter::new(file)
        .finish(&mut df)
        .map_err(ExportError::Polars)?;
    Ok(())
}

/// Write a DataFrame to Parquet.
///
/// Polars 0.46 does not expose custom footer key-value metadata through its
/// public API, so provenance is written as a sidecar JSON file at
/// `<stem>.provenance.json`.  The file name suffix makes it machine-readable
/// alongside the parquet file.
pub fn write_parquet(df: DataFrame, path: &Path, provenance: &Provenance) -> Result<(), ExportError> {
    write_parquet_plain(df, path)?;
    write_provenance_sidecar(path, provenance)
}

/// Write a DataFrame to TSV (tab-separated) with header row first.
pub fn write_tsv_plain(mut df: DataFrame, path: &Path) -> Result<(), ExportError> {
    let file = std::fs::File::create(path)?;
    CsvWriter::new(file)
        .with_separator(b'\t')
        .finish(&mut df)
        .map_err(ExportError::Polars)
}

// ---------------------------------------------------------------------------
// XLSX
// ---------------------------------------------------------------------------

/// Write a DataFrame to XLSX (data sheet only — no provenance sheet).
pub fn write_xlsx_plain(df: DataFrame, path: &Path) -> Result<(), ExportError> {
    use rust_xlsxwriter::Workbook;

    let mut wb = Workbook::new();
    let headers: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    let n_rows = df.height();

    let ws = wb.add_worksheet();
    ws.set_name("data")
        .map_err(|e| ExportError::Xlsx(e.to_string()))?;

    for (col_idx, name) in headers.iter().enumerate() {
        ws.write_string(0, col_idx as u16, name)
            .map_err(|e| ExportError::Xlsx(e.to_string()))?;
    }

    for (col_idx, col) in df.get_columns().iter().enumerate() {
        for row_idx in 0..n_rows {
            let xrow = (row_idx + 1) as u32;
            let xcol = col_idx as u16;
            let val = col.get(row_idx).unwrap_or(AnyValue::Null);
            write_any_value(ws, xrow, xcol, val)?;
        }
    }

    wb.save(path).map_err(|e| ExportError::Xlsx(e.to_string()))
}

/// Write a DataFrame to XLSX with a `_provenance` metadata sheet.
pub fn write_xlsx(df: DataFrame, path: &Path, provenance: &Provenance) -> Result<(), ExportError> {
    use rust_xlsxwriter::Workbook;

    let mut wb = Workbook::new();
    let headers: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    let n_rows = df.height();

    // ── Data sheet ────────────────────────────────────────────────────────
    {
        let ws = wb.add_worksheet();
        ws.set_name("data")
            .map_err(|e| ExportError::Xlsx(e.to_string()))?;

        for (col_idx, name) in headers.iter().enumerate() {
            ws.write_string(0, col_idx as u16, name)
                .map_err(|e| ExportError::Xlsx(e.to_string()))?;
        }

        for (col_idx, col) in df.get_columns().iter().enumerate() {
            for row_idx in 0..n_rows {
                let xrow = (row_idx + 1) as u32;
                let xcol = col_idx as u16;
                let val = col.get(row_idx).unwrap_or(AnyValue::Null);
                write_any_value(ws, xrow, xcol, val)?;
            }
        }
    }

    // ── Provenance sheet ──────────────────────────────────────────────────
    {
        let ws = wb.add_worksheet();
        ws.set_name("_provenance")
            .map_err(|e| ExportError::Xlsx(e.to_string()))?;

        let fields: &[(&str, &str)] = &[
            ("build",          provenance.build.stamp()),
            ("app_version",    &provenance.app_version),
            ("schema_version", &provenance.schema_version),
            ("in_dsn",         &provenance.in_dsn),
            ("timestamp",      &provenance.timestamp),
        ];
        for (i, (key, val)) in fields.iter().enumerate() {
            ws.write_string(i as u32, 0, *key)
                .map_err(|e| ExportError::Xlsx(e.to_string()))?;
            ws.write_string(i as u32, 1, *val)
                .map_err(|e| ExportError::Xlsx(e.to_string()))?;
        }
    }

    wb.save(path).map_err(|e| ExportError::Xlsx(e.to_string()))
}

fn write_any_value(
    ws: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    val: AnyValue<'_>,
) -> Result<(), ExportError> {
    // Prefer typed writers. For strings use str_value() — AnyValue's Display
    // wraps Utf8 values in quotes (e.g. `"alive"`), which would land in the cell.
    let r = match val {
        AnyValue::Null => return Ok(()),
        AnyValue::Float64(v) => ws.write_number(row, col, v),
        AnyValue::Float32(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::Int64(v) => ws.write_number(row, col, v as f64),
        AnyValue::Int32(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::Int16(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::Int8(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::UInt64(v) => ws.write_number(row, col, v as f64),
        AnyValue::UInt32(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::UInt16(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::UInt8(v) => ws.write_number(row, col, f64::from(v)),
        AnyValue::Boolean(v) => ws.write_boolean(row, col, v),
        AnyValue::String(_) | AnyValue::StringOwned(_) => {
            ws.write_string(row, col, val.str_value().as_ref())
        }
        other => ws.write_string(row, col, other.str_value().as_ref()),
    };
    r.map(|_| ()).map_err(|e| ExportError::Xlsx(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xlsx_string_cells_have_no_polars_display_quotes() {
        use calamine::Reader;

        let df = DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1", "P2"])),
            Column::from(Series::new("Status".into(), &["alive", "snag"])),
        ])
        .unwrap();
        let path = std::env::temp_dir().join("gfb3_xlsx_quote_test.xlsx");
        write_xlsx(df, &path, &Provenance::new_draft("test")).unwrap();

        let mut wb = calamine::open_workbook_auto(&path).unwrap();
        let range = wb.worksheet_range("data").unwrap();
        let _ = std::fs::remove_file(&path);

        let p1 = range.get((1, 0)).unwrap().to_string();
        let alive = range.get((1, 1)).unwrap().to_string();
        assert_eq!(p1, "P1", "PlotID cell should be bare P1, got {p1:?}");
        assert_eq!(alive, "alive", "Status cell should be bare alive, got {alive:?}");
        assert!(!p1.starts_with('"') && !alive.starts_with('"'));
    }
}
