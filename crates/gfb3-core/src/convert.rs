//! File-format conversion between rectangular table formats.
//!
//! Reads any supported input (CSV / TSV / TXT / XLSX / Parquet) via [`crate::reader`]
//! and writes CSV, TSV, Parquet, and/or XLSX without draft provenance stamps —
//! this is a utility conversion, not a GFB3 export.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::export::{
    write_csv_plain, write_parquet_plain, write_tsv_plain, write_xlsx_plain, ExportError,
};
use crate::reader::{read_file, ReadError};

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("{0}")]
    Read(#[from] ReadError),
    #[error("{0}")]
    Write(#[from] ExportError),
    #[error("unsupported output format '{0}'; expected csv, tsv, parquet, or xlsx")]
    UnsupportedFormat(String),
    #[error("no output formats selected")]
    NoFormats,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Output table formats supported by the converter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableFormat {
    Csv,
    Tsv,
    Parquet,
    Xlsx,
}

impl TableFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            TableFormat::Csv => "csv",
            TableFormat::Tsv => "tsv",
            TableFormat::Parquet => "parquet",
            TableFormat::Xlsx => "xlsx",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.trim().trim_start_matches('.').to_lowercase().as_str() {
            "csv" => Some(TableFormat::Csv),
            "tsv" => Some(TableFormat::Tsv),
            "parquet" => Some(TableFormat::Parquet),
            "xlsx" | "xls" => Some(TableFormat::Xlsx),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Result<Self, ConvertError> {
        Self::from_extension(s).ok_or_else(|| ConvertError::UnsupportedFormat(s.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertResult {
    pub input: String,
    pub outputs: Vec<String>,
    pub row_count: usize,
    pub column_count: usize,
}

/// Convert one table file to a single output path (format taken from the
/// destination extension).
pub fn convert_file(input: &Path, output: &Path) -> Result<ConvertResult, ConvertError> {
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let format = TableFormat::parse(ext)?;
    convert_to_formats(input, output.parent().unwrap_or(Path::new(".")), stem_name(output), &[format])
}

/// Convert one table file into one or more formats under `out_dir`.
///
/// Output files are named `{base_name}.{ext}` (e.g. `plots.parquet`).
pub fn convert_to_formats(
    input: &Path,
    out_dir: &Path,
    base_name: &str,
    formats: &[TableFormat],
) -> Result<ConvertResult, ConvertError> {
    if formats.is_empty() {
        return Err(ConvertError::NoFormats);
    }

    let df = read_file(input)?;
    let row_count = df.height();
    let column_count = df.width();

    std::fs::create_dir_all(out_dir)?;

    let mut outputs = Vec::with_capacity(formats.len());
    let mut unique = std::collections::HashSet::new();
    for fmt in formats {
        if !unique.insert(*fmt) {
            continue;
        }
        let path = out_dir.join(format!("{base_name}.{}", fmt.extension()));
        match fmt {
            TableFormat::Csv => write_csv_plain(df.clone(), &path)?,
            TableFormat::Tsv => write_tsv_plain(df.clone(), &path)?,
            TableFormat::Parquet => write_parquet_plain(df.clone(), &path)?,
            TableFormat::Xlsx => write_xlsx_plain(df.clone(), &path)?,
        }
        outputs.push(path.to_string_lossy().into_owned());
    }

    Ok(ConvertResult {
        input: input.to_string_lossy().into_owned(),
        outputs,
        row_count,
        column_count,
    })
}

fn stem_name(path: &Path) -> &str {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn csv_to_parquet_roundtrip_shape() {
        let dir = std::env::temp_dir().join("gfb3_convert_test");
        let _ = std::fs::create_dir_all(&dir);
        let csv_path = dir.join("in.csv");
        {
            let mut f = std::fs::File::create(&csv_path).unwrap();
            writeln!(f, "PlotID,YR,DBH\nP1,2010,12.5\nP1,2011,13.0").unwrap();
        }
        let out = convert_to_formats(
            &csv_path,
            &dir,
            "out",
            &[TableFormat::Parquet, TableFormat::Xlsx, TableFormat::Tsv],
        )
        .expect("convert");
        assert_eq!(out.row_count, 2);
        assert_eq!(out.column_count, 3);
        assert_eq!(out.outputs.len(), 3);
        assert!(dir.join("out.parquet").exists());
        assert!(dir.join("out.xlsx").exists());
        assert!(dir.join("out.tsv").exists());

        let back = convert_file(&dir.join("out.parquet"), &dir.join("back.csv")).unwrap();
        assert_eq!(back.row_count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_empty_formats() {
        let err = convert_to_formats(
            Path::new("x.csv"),
            Path::new("."),
            "x",
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, ConvertError::NoFormats));
    }
}
