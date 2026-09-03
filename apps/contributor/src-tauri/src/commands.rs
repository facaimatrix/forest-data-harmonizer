use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::command;

use gfb3_core::export::{
    coerce_status_to_int, dataset_summary_filename, draft_filename, gfb3_draft_filename,
    plots_summary_filename, write_csv, write_parquet, write_xlsx, Provenance,
};
use gfb3_core::gfb2::gfb3_to_gfb2;
use gfb3_core::log::CurationLog;
use gfb3_core::mapping::{
    CensusType, ColumnMapping, ContributorMapping, DatasetMetadata, DbhUnit, StatusRemap,
};
use gfb3_core::reader::{dataframe_preview, read_file};
use gfb3_core::schema::{
    gfb2_export_columns, gfb3_export_columns, select_export_columns, with_expan, with_plot_yr,
    ExpanSpec, GateErrorItem, Gfb3Field, InputGate,
};
use gfb3_core::summary::{build_dataset_summary, build_plots_summary};
use gfb3_core::tnrs::{
    build_species_entries, build_tnrs_request, parse_tnrs_response, tnrs_url, TnrsResultRow,
};
use gfb3_core::transform::{
    apply_column_mapping, apply_field_exprs, apply_status_remap, derive_status_column,
    melt_wide_to_long, prepare_mapped_frame, scale_dbh_mm_to_cm, DeriveStatusSummary, FieldExpr,
};
use gfb3_core::diagnostic::{
    build_diagnostic_report, write_diagnostic_html, write_diagnostic_pdf, DiagnosticReport,
};
use gfb3_core::validation::{
    drop_anchor_rows, drop_invalid_rows, nullify_dead_dbh, recode_unknown_status, sort_for_lag,
    validate, ValidateOptions, ValidationReport,
};
use polars::prelude::{AnyValue, DataFrame, DataType, IntoLazy, JoinArgs, JoinType, col as pcol};

use crate::state::{AppState, SessionState};

// ---------------------------------------------------------------------------
// Step 0: Load file + structural gate
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct LoadResult {
    pub columns: Vec<String>,
    pub row_count: usize,
    /// First 5 rows, row-major, values as strings (nulls → null in JSON).
    pub preview_rows: Vec<Vec<Option<String>>>,
    /// Plain-language structural gate errors (empty = passed).
    pub gate_errors: Vec<GateErrorItem>,
    /// Fuzzy-suggested column mappings for the mapping step.
    pub suggested_mappings: Vec<SuggestedMapping>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedMapping {
    pub source_column: String,
    /// None when no suggestion could be made.
    pub suggested_gfb3_field: Option<String>,
}

#[command]
pub async fn load_file(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<LoadResult, String> {
    let file_path = std::path::Path::new(&path);
    let df = read_file(file_path).map_err(|e| e.to_string())?;

    let gate_errors: Vec<GateErrorItem> = InputGate::check(&df)
        .into_iter()
        .map(|e| e.item())
        .collect();

    let columns: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let suggestions = ContributorMapping::suggest_from_headers(&columns);
    let meta_suggestions = ContributorMapping::suggest_plot_meta_from_headers(&columns);
    let suggested_mappings = columns
        .iter()
        .map(|col| {
            let field = suggestions
                .iter()
                .find(|m| &m.source_column == col)
                .map(|m| format!("{:?}", m.target_field))
                .or_else(|| {
                    meta_suggestions
                        .iter()
                        .find(|(src, _)| src == col)
                        .map(|(_, tgt)| tgt.clone())
                });
            SuggestedMapping {
                source_column: col.clone(),
                suggested_gfb3_field: field,
            }
        })
        .collect();

    let preview_rows = dataframe_preview(&df, 5);
    let row_count = df.height();

    *state.session.lock().unwrap() = Some(SessionState {
        raw_df: df,
        file_path: path,
        mapped_df: None,
        mapping: None,
        validation_report: None,
        diagnostic_report: None,
    });

    Ok(LoadResult {
        columns,
        row_count,
        preview_rows,
        gate_errors,
        suggested_mappings,
    })
}

// ---------------------------------------------------------------------------
// Step 2–4: Column mapping + metadata + status vocabulary
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ColumnMappingInput {
    pub source_column: String,
    /// Valid Gfb3Field debug names: "PlotId", "TreeId", "Yr", "PrevYr",
    /// "Status", "Dbh", "Species", "Dsn".
    pub target_field: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusRemapInput {
    pub source_value: String,
    /// GFB3 code: "0", "1", "2", or "9".
    pub target_code: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataInput {
    pub country: Option<String>,
    pub site: Option<String>,
    pub pi: Option<String>,
    pub pi_email: Option<String>,
    pub contact: Option<String>,
    pub contact_email: Option<String>,
    /// "cm" or "mm"
    pub dbh_unit: Option<String>,
    pub census_years: Vec<u32>,
    /// "single" or "multi" (defaults to multi)
    pub census_type: Option<String>,
}

fn metadata_from_input(
    meta: MetadataInput,
    dbh_unit: Option<DbhUnit>,
    census_type: CensusType,
) -> DatasetMetadata {
    DatasetMetadata {
        country: meta.country,
        site: meta.site,
        pi: meta.pi,
        pi_email: meta.pi_email,
        contact: meta.contact,
        contact_email: meta.contact_email,
        dbh_unit,
        coordinate_crs: None,
        census_years: meta.census_years,
        census_type,
    }
}

#[derive(Debug, Deserialize)]
pub struct ApplyMappingRequest {
    pub gfb3_dsn: String,
    pub column_mappings: Vec<ColumnMappingInput>,
    pub status_remaps: Vec<StatusRemapInput>,
    pub metadata: MetadataInput,
}

#[derive(Debug, Serialize)]
pub struct ApplyMappingResult {
    pub mapped_columns: Vec<String>,
    pub row_count: usize,
}

#[command]
pub async fn apply_mapping(
    state: tauri::State<'_, AppState>,
    request: ApplyMappingRequest,
) -> Result<ApplyMappingResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded — call load_file first")?;

    let col_mappings: Vec<ColumnMapping> = request
        .column_mappings
        .into_iter()
        .map(|m| {
            let field = parse_gfb3_field(&m.target_field)
                .ok_or_else(|| format!("unknown GFB3 field '{}'", m.target_field))?;
            Ok(ColumnMapping {
                source_column: m.source_column,
                target_field: field,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let status_remaps: Vec<StatusRemap> = request
        .status_remaps
        .into_iter()
        .map(|r| StatusRemap {
            source_value: r.source_value,
            target_code: r.target_code,
            note: r.note,
        })
        .collect();

    let dbh_unit = match request.metadata.dbh_unit.as_deref() {
        Some("mm") => Some(DbhUnit::Mm),
        Some("cm") | None => Some(DbhUnit::Cm),
        Some(other) => {
            return Err(format!("unknown DBH unit '{other}'; expected 'cm' or 'mm'"))
        }
    };

    let census_type = parse_census_type(&request.metadata);
    let metadata = metadata_from_input(request.metadata, dbh_unit, census_type);

    let mapping = ContributorMapping {
        gfb3_dsn: request.gfb3_dsn.clone(),
        column_mappings: col_mappings.clone(),
        status_remaps: status_remaps.clone(),
        needs_pivot: false,
        wide_dbh_columns: vec![],
        metadata,
    };

    let lf = session.raw_df.clone().lazy();
    let lf = apply_column_mapping(lf, &col_mappings, &request.gfb3_dsn);

    let remap_pairs: Vec<(String, String)> = status_remaps
        .iter()
        .map(|r| (r.source_value.clone(), r.target_code.clone()))
        .collect();
    let lf = apply_status_remap(lf, &remap_pairs);

    let lf = if matches!(mapping.metadata.dbh_unit, Some(DbhUnit::Mm)) {
        scale_dbh_mm_to_cm(lf)
    } else {
        lf
    };

    let mapped_df = lf.collect().map_err(|e| e.to_string())?;

    let mapped_columns: Vec<String> = mapped_df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let row_count = mapped_df.height();

    session.mapped_df = Some(mapped_df);
    session.mapping = Some(mapping);
    session.validation_report = None;
    session.diagnostic_report = None;

    Ok(ApplyMappingResult { mapped_columns, row_count })
}

// ---------------------------------------------------------------------------
// Step 4: Status vocabulary editor
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StatusVocabRow {
    pub source_value: String,
    pub current_target: String,
    pub row_count: usize,
}

/// Return distinct Status values + their current remap target so the UI can
/// show the status-vocabulary editor (step 5).
#[command]
pub async fn get_status_vocab(
    state: tauri::State<'_, AppState>,
    column: Option<String>,
) -> Result<Vec<StatusVocabRow>, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("no file loaded")?;

    if let Some(col_name) = column.filter(|c| !c.is_empty()) {
        return status_vocab_counts(&session.raw_df, &col_name, session.mapping.as_ref());
    }

    // Prefer the already-mapped DataFrame; fall back to raw frame.
    let (df, col_name) = if let Some(mapped) = &session.mapped_df {
        let name = if mapped.get_column_names().iter().any(|n| n.as_str() == "Status") {
            "Status".to_string()
        } else {
            return Err("Status column not found in mapped DataFrame".into());
        };
        (mapped, name)
    } else {
        let name = session
            .mapping
            .as_ref()
            .and_then(|m| {
                m.column_mappings
                    .iter()
                    .find(|c| c.target_field == Gfb3Field::Status)
                    .map(|c| c.source_column.clone())
            })
            .unwrap_or_else(|| "Status".to_string());
        (&session.raw_df, name)
    };

    status_vocab_counts(df, &col_name, session.mapping.as_ref())
}

fn status_vocab_counts(
    df: &DataFrame,
    col_name: &str,
    mapping: Option<&ContributorMapping>,
) -> Result<Vec<StatusVocabRow>, String> {
    let col = df
        .column(col_name)
        .map_err(|_| format!("column '{col_name}' not found — complete column-mapping step first"))?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for i in 0..col.len() {
        let val = col.get(i).unwrap_or(AnyValue::Null);
        let s = match val {
            AnyValue::Null => continue,
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.str_value().into_owned(),
        };
        if s.trim().is_empty() || s == "null" {
            continue;
        }
        *counts.entry(s).or_insert(0) += 1;
    }

    let mut rows: Vec<StatusVocabRow> = counts
        .into_iter()
        .map(|(source_value, row_count)| {
            let current_target = mapping
                .map(|m| m.remap_status(&source_value).to_string())
                .unwrap_or_else(|| "9".to_string());
            StatusVocabRow { source_value, current_target, row_count }
        })
        .collect();

    rows.sort_by(|a, b| b.row_count.cmp(&a.row_count));
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Step 5: Validation + diagnostic report
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ValidateStepResult {
    pub validation: ValidationReport,
    pub diagnostic: DiagnosticReport,
}

#[command]
pub async fn run_validation(
    state: tauri::State<'_, AppState>,
    locale: Option<String>,
) -> Result<ValidateStepResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded")?;

    let df = session
        .mapped_df
        .as_ref()
        .ok_or("column mapping not applied — call apply_mapping first")?;

    let census_type = session
        .mapping
        .as_ref()
        .map(|m| m.metadata.census_type)
        .unwrap_or_else(|| {
            // Diagnose mode: infer from lag columns when no mapping metadata exists.
            let names = df.get_column_names();
            let has_prev = names.iter().any(|c| c.as_str() == "PrevYR")
                || names.iter().any(|c| c.as_str() == "PrevDBH");
            if has_prev {
                CensusType::Multi
            } else {
                CensusType::Single
            }
        });

    let dataset_name = session
        .mapping
        .as_ref()
        .map(|m| m.gfb3_dsn.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            std::path::Path::new(&session.file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("dataset")
                .to_string()
        });

    let report = validate(
        df.clone().lazy(),
        ValidateOptions { census_type },
    )
    .map_err(|e| e.to_string())?;

    let diagnostic = build_diagnostic_report(
        df,
        census_type,
        &dataset_name,
        None,
        None,
        locale.as_deref().unwrap_or("en"),
    )
    .map_err(|e| e.to_string())?;

    session.validation_report = Some(report.clone());
    session.diagnostic_report = Some(diagnostic.clone());
    Ok(ValidateStepResult {
        validation: report,
        diagnostic,
    })
}

#[derive(Debug, Deserialize)]
pub struct DiagnosticExportRequest {
    pub path: String,
    /// "pdf" | "html"
    pub format: String,
    /// UI locale for PDF text (en / es / pt).
    #[serde(default)]
    pub locale: String,
}

#[command]
pub async fn export_diagnostic_report(
    state: tauri::State<'_, AppState>,
    request: DiagnosticExportRequest,
) -> Result<String, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("no file loaded")?;
    let report = session
        .diagnostic_report
        .as_ref()
        .ok_or("diagnostic report not available — run validation first")?;

    let path = std::path::Path::new(&request.path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let locale = if request.locale.trim().is_empty() {
        "en"
    } else {
        request.locale.trim()
    };

    match request.format.as_str() {
        "pdf" => write_diagnostic_pdf(report, path, locale)?,
        "html" => write_diagnostic_html(report, path).map_err(|e| e.to_string())?,
        other => return Err(format!("unsupported diagnostic export format: {other}")),
    }
    Ok(request.path)
}

// ---------------------------------------------------------------------------
// Step 6: Export
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub output_dir: String,
    pub base_name: String,
    pub formats: Vec<String>,
    /// Multi-census: drop Status ≠ 0 before GFB2 export (default true).
    #[serde(default = "default_true")]
    pub keep_alive_only: bool,
    /// Fixed-area plots → EXPAN = 1/PA (default true).
    #[serde(default = "default_true")]
    pub fixed_area: bool,
    /// When `fixed_area` is false: Some(v) fills EXPAN with a constant;
    /// None leaves EXPAN blank ("add later").
    #[serde(default)]
    pub constant_expan: Option<f64>,
    /// Curator of record for the curation-log skeleton.
    #[serde(default)]
    pub curator: String,
    /// UI locale for exported curation log (en / es / pt).
    #[serde(default)]
    pub locale: String,
}

fn default_true() -> bool {
    true
}

#[command]
pub async fn export(
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<Vec<String>, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("no file loaded")?;

    let mapped_df = session
        .mapped_df
        .as_ref()
        .ok_or("column mapping not applied — complete steps 1–4 first")?;

    let dsn = session
        .mapping
        .as_ref()
        .map(|m| m.gfb3_dsn.as_str())
        .unwrap_or("dataset");

    let census_type = session
        .mapping
        .as_ref()
        .map(|m| m.metadata.census_type)
        .unwrap_or(CensusType::Multi);

    // Shared cleaning before any format branch.
    let cleaned = {
        let lf = mapped_df.clone().lazy();
        let lf = recode_unknown_status(lf);
        let lf = nullify_dead_dbh(lf);
        let lf = drop_invalid_rows(lf, census_type);
        sort_for_lag(lf)
    };

    // Keep a Status-bearing frame for summaries (before GFB2 strips Status).
    let cleaned_df = cleaned.collect().map_err(|e| e.to_string())?;
    let expan_spec = if request.fixed_area {
        ExpanSpec::FixedArea
    } else if let Some(v) = request.constant_expan {
        ExpanSpec::Constant(v)
    } else {
        ExpanSpec::Blank
    };
    let cleaned_df = with_expan(cleaned_df, expan_spec).map_err(|e| e.to_string())?;

    let gfb2_df = gfb3_to_gfb2(cleaned_df.clone().lazy(), request.keep_alive_only)
        .map_err(|e| e.to_string())?;

    let selected: Vec<&str> = request
        .formats
        .iter()
        .filter_map(|f| match f.as_str() {
            "csv" | "parquet" | "xlsx" => Some(f.as_str()),
            // Legacy aliases from older UI — treat as the base format.
            "gfb3-csv" | "gfb3_csv" => Some("csv"),
            "gfb3-parquet" | "gfb3_parquet" => Some("parquet"),
            "gfb3-xlsx" | "gfb3_xlsx" => Some("xlsx"),
            "fisi" => None, // removed product
            _ => None,
        })
        .collect();
    // Deduplicate while preserving order.
    let mut formats: Vec<&str> = Vec::new();
    for f in selected {
        if !formats.contains(&f) {
            formats.push(f);
        }
    }
    if formats.is_empty() {
        if let Some(u) = request.formats.iter().find(|f| {
            !matches!(
                f.as_str(),
                "csv"
                    | "parquet"
                    | "xlsx"
                    | "fisi"
                    | "gfb3-csv"
                    | "gfb3_csv"
                    | "gfb3-parquet"
                    | "gfb3_parquet"
                    | "gfb3-xlsx"
                    | "gfb3_xlsx"
            )
        }) {
            return Err(format!("unknown format '{u}'"));
        }
        return Err("select at least one format (csv, parquet, or xlsx)".into());
    }

    // Paired-census GFB3 (multi only)
    let gfb3_df = if census_type == CensusType::Multi {
        let lf = drop_anchor_rows(cleaned_df.clone().lazy());
        let lf = coerce_status_to_int(lf);
        let df = lf.collect().map_err(|e| e.to_string())?;
        let df = with_plot_yr(df).map_err(|e| e.to_string())?;
        Some(select_export_columns(df, gfb3_export_columns()).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let provenance = Provenance::new_draft(dsn);
    let out_dir = std::path::Path::new(&request.output_dir);
    let mut written: Vec<String> = Vec::new();

    let gfb2_write = select_export_columns(gfb2_df, gfb2_export_columns())
        .map_err(|e| e.to_string())?;

    let empty_meta = DatasetMetadata::default();
    let meta_ref = session
        .mapping
        .as_ref()
        .map(|m| &m.metadata)
        .unwrap_or(&empty_meta);
    let plots_df = build_plots_summary(&cleaned_df).map_err(|e| e.to_string())?;
    let dataset_df =
        build_dataset_summary(&cleaned_df, meta_ref, dsn).map_err(|e| e.to_string())?;

    for fmt in formats {
        // Harmonized tree table (GFB2)
        match fmt {
            "csv" => {
                let path = out_dir.join(draft_filename(&request.base_name, "csv"));
                write_csv(gfb2_write.clone(), &path, &provenance)
                    .map_err(|e| format!("gfb2 csv export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "parquet" => {
                let path = out_dir.join(draft_filename(&request.base_name, "parquet"));
                write_parquet(gfb2_write.clone(), &path, &provenance)
                    .map_err(|e| format!("gfb2 parquet export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "xlsx" => {
                let path = out_dir.join(draft_filename(&request.base_name, "xlsx"));
                write_xlsx(gfb2_write.clone(), &path, &provenance)
                    .map_err(|e| format!("gfb2 xlsx export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            _ => {}
        }

        // plots_summary
        match fmt {
            "csv" => {
                let path = out_dir.join(plots_summary_filename(&request.base_name, "csv"));
                write_csv(plots_df.clone(), &path, &provenance)
                    .map_err(|e| format!("plots_summary csv export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "parquet" => {
                let path = out_dir.join(plots_summary_filename(&request.base_name, "parquet"));
                write_parquet(plots_df.clone(), &path, &provenance)
                    .map_err(|e| format!("plots_summary parquet export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "xlsx" => {
                let path = out_dir.join(plots_summary_filename(&request.base_name, "xlsx"));
                write_xlsx(plots_df.clone(), &path, &provenance)
                    .map_err(|e| format!("plots_summary xlsx export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            _ => {}
        }

        // dataset_summary
        match fmt {
            "csv" => {
                let path = out_dir.join(dataset_summary_filename(&request.base_name, "csv"));
                write_csv(dataset_df.clone(), &path, &provenance)
                    .map_err(|e| format!("dataset_summary csv export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "parquet" => {
                let path = out_dir.join(dataset_summary_filename(&request.base_name, "parquet"));
                write_parquet(dataset_df.clone(), &path, &provenance)
                    .map_err(|e| format!("dataset_summary parquet export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            "xlsx" => {
                let path = out_dir.join(dataset_summary_filename(&request.base_name, "xlsx"));
                write_xlsx(dataset_df.clone(), &path, &provenance)
                    .map_err(|e| format!("dataset_summary xlsx export failed: {e}"))?;
                written.push(path.to_string_lossy().into_owned());
            }
            _ => {}
        }

        // GFB3 paired-census (multi only)
        if let Some(df) = gfb3_df.as_ref() {
            match fmt {
                "csv" => {
                    let path = out_dir.join(gfb3_draft_filename(&request.base_name, "csv"));
                    write_csv(df.clone(), &path, &provenance)
                        .map_err(|e| format!("gfb3 csv export failed: {e}"))?;
                    written.push(path.to_string_lossy().into_owned());
                }
                "parquet" => {
                    let path = out_dir.join(gfb3_draft_filename(&request.base_name, "parquet"));
                    write_parquet(df.clone(), &path, &provenance)
                        .map_err(|e| format!("gfb3 parquet export failed: {e}"))?;
                    written.push(path.to_string_lossy().into_owned());
                }
                "xlsx" => {
                    let path = out_dir.join(gfb3_draft_filename(&request.base_name, "xlsx"));
                    write_xlsx(df.clone(), &path, &provenance)
                        .map_err(|e| format!("gfb3 xlsx export failed: {e}"))?;
                    written.push(path.to_string_lossy().into_owned());
                }
                _ => {}
            }
        }
    }

    // Curation log skeleton
    let curator = if request.curator.trim().is_empty() {
        "Unknown"
    } else {
        request.curator.trim()
    };
    let mut log = CurationLog::new(curator);
    if let Some(mapping) = &session.mapping {
        log.prefill_from_metadata(&mapping.metadata, &mapping.gfb3_dsn);
    }
    if let Some(report) = &session.validation_report {
        log.append_escalated_findings(&report.findings);
    }
    let log_path = out_dir.join(format!("{}_curation_log.txt", request.base_name));
    let loc = if request.locale.trim().is_empty() {
        "en"
    } else {
        request.locale.trim()
    };
    std::fs::write(&log_path, log.render_with_locale(loc)).map_err(|e| e.to_string())?;
    written.push(log_path.to_string_lossy().into_owned());

    Ok(written)
}

// ---------------------------------------------------------------------------
// Wide-format pivot mapping
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct WidePair {
    pub source_column: String,
    pub year: u32,
}

#[derive(Debug, Deserialize)]
pub struct ApplyWideMappingRequest {
    pub gfb3_dsn: String,
    /// Field expressions for identity columns (PlotID, TreeID, Species, Status, Lat, Lon, PA).
    /// Supports single-column rename, literal, and concat (same as apply_fields_mapping).
    pub identity_exprs: Vec<FieldExprInput>,
    /// Each census DBH column paired with its year.
    pub dbh_pairs: Vec<WidePair>,
    pub status_remaps: Vec<StatusRemapInput>,
    pub metadata: MetadataInput,
}

#[command]
pub async fn apply_wide_mapping(
    state: tauri::State<'_, AppState>,
    request: ApplyWideMappingRequest,
) -> Result<ApplyMappingResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded — call load_file first")?;

    // Apply lookup joins first, then regular identity field exprs.
    let lf = apply_lookup_joins(session.raw_df.clone().lazy(), &request.identity_exprs)
        .map_err(|e| e)?;

    let exprs: Vec<FieldExpr> = request.identity_exprs.iter()
        .filter(|f| f.kind != "lookup")
        .map(field_expr_from_input)
        .collect();

    let lf = apply_field_exprs(lf, &exprs, &request.gfb3_dsn)
        .map_err(|e| e.to_string())?;
    let renamed_df = lf.collect().map_err(|e| e.to_string())?;

    // id_cols = target column names from the exprs + in_dsn.
    let mut id_cols: Vec<String> = exprs.iter().map(|e| match e {
        FieldExpr::Column  { target_col, .. } => target_col.clone(),
        FieldExpr::Literal { target_col, .. } => target_col.clone(),
        FieldExpr::Concat  { target_col, .. } => target_col.clone(),
        FieldExpr::YearFromColumn { target_col, .. } => target_col.clone(),
    }).collect();
    id_cols.push("in_dsn".to_string());

    let id_refs: Vec<&str> = id_cols.iter().map(|s| s.as_str()).collect();
    let pairs_ref: Vec<(&str, u32)> = request
        .dbh_pairs
        .iter()
        .map(|p| (p.source_column.as_str(), p.year))
        .collect();

    let long_df = melt_wide_to_long(&renamed_df, &id_refs, &pairs_ref)
        .map_err(|e| e.to_string())?;

    let status_remaps = status_remaps_from_input(&request.status_remaps);
    let remap_pairs: Vec<(String, String)> = status_remaps
        .iter()
        .map(|r| (r.source_value.clone(), r.target_code.clone()))
        .collect();

    let dbh_unit = match request.metadata.dbh_unit.as_deref() {
        Some("mm") => Some(DbhUnit::Mm),
        Some("cm") | None => Some(DbhUnit::Cm),
        Some(other) => return Err(format!("unknown DBH unit '{other}'; expected 'cm' or 'mm'")),
    };

    let census_type = parse_census_type(&request.metadata);

    let lf = long_df.lazy();
    let lf = apply_status_remap(lf, &remap_pairs);
    let lf = if matches!(dbh_unit, Some(DbhUnit::Mm)) { scale_dbh_mm_to_cm(lf) } else { lf };
    let lf = prepare_mapped_frame(lf, census_type);

    let mapped_df = lf.collect().map_err(|e| e.to_string())?;
    let mapped_columns: Vec<String> =
        mapped_df.get_column_names().iter().map(|s| s.to_string()).collect();
    let row_count = mapped_df.height();

    let metadata = metadata_from_input(request.metadata, dbh_unit, census_type);
    let mapping = ContributorMapping {
        gfb3_dsn: request.gfb3_dsn.clone(),
        column_mappings: vec![],
        status_remaps,
        needs_pivot: false,
        wide_dbh_columns: vec![],
        metadata,
    };

    session.mapped_df = Some(mapped_df);
    session.mapping = Some(mapping);
    session.validation_report = None;
    session.diagnostic_report = None;

    Ok(ApplyMappingResult { mapped_columns, row_count })
}

// ---------------------------------------------------------------------------
// Field-wizard mapping (new per-field approach)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct FieldExprInput {
    pub kind:        String,
    pub target_col:  String,
    // column / concat fields
    pub source:      Option<String>,
    pub value:       Option<String>,
    pub sources:     Option<Vec<String>>,
    pub sep:         Option<String>,
    pub to_lower:    Option<bool>,
    pub prefix:      Option<String>,
    // lookup fields
    pub lookup_path: Option<String>,
    pub main_key:    Option<String>,
    pub lookup_key:  Option<String>,
    pub value_col:   Option<String>,
}

/// Load a file and return its column names only. Does NOT modify session state.
#[command]
pub async fn preview_file(path: String) -> Result<Vec<String>, String> {
    let df = read_file(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(df.get_column_names().iter().map(|s| s.to_string()).collect())
}

#[derive(Debug, Deserialize)]
pub struct ApplyFieldsRequest {
    pub gfb3_dsn:     String,
    pub fields:       Vec<FieldExprInput>,
    pub dbh_unit:     Option<String>,
    pub status_remaps: Vec<StatusRemapInput>,
    pub metadata:     MetadataInput,
}

#[command]
pub async fn apply_fields_mapping(
    state: tauri::State<'_, AppState>,
    request: ApplyFieldsRequest,
) -> Result<ApplyMappingResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded — call load_file first")?;

    let lf = apply_lookup_joins(session.raw_df.clone().lazy(), &request.fields)
        .map_err(|e| e)?;

    let exprs: Vec<FieldExpr> = request.fields.iter()
        .filter(|f| f.kind != "lookup")
        .map(field_expr_from_input)
        .collect();

    let lf = apply_field_exprs(lf, &exprs, &request.gfb3_dsn)
        .map_err(|e| e.to_string())?;

    let status_remaps = status_remaps_from_input(&request.status_remaps);
    let remap_pairs: Vec<(String, String)> = status_remaps
        .iter()
        .map(|r| (r.source_value.clone(), r.target_code.clone()))
        .collect();
    let lf = if !remap_pairs.is_empty() { apply_status_remap(lf, &remap_pairs) } else { lf };

    let dbh_unit = match request.dbh_unit.as_deref() {
        Some("mm") => { let lf_scaled = scale_dbh_mm_to_cm(lf); (lf_scaled, Some(DbhUnit::Mm)) }
        _          => (lf, Some(DbhUnit::Cm)),
    };
    let (lf, dbh_unit_enum) = dbh_unit;

    let census_type = parse_census_type(&request.metadata);
    let lf = prepare_mapped_frame(lf, census_type);

    let mapped_df = lf.collect().map_err(|e| e.to_string())?;
    let mapped_columns: Vec<String> = mapped_df.get_column_names().iter().map(|s| s.to_string()).collect();
    let row_count = mapped_df.height();

    let metadata = metadata_from_input(request.metadata, dbh_unit_enum, census_type);
    let mapping = ContributorMapping {
        gfb3_dsn:        request.gfb3_dsn,
        column_mappings: vec![],
        status_remaps,
        needs_pivot:     false,
        wide_dbh_columns: vec![],
        metadata,
    };

    session.mapped_df = Some(mapped_df);
    session.mapping = Some(mapping);
    session.validation_report = None;
    session.diagnostic_report = None;

    Ok(ApplyMappingResult { mapped_columns, row_count })
}

// ---------------------------------------------------------------------------
// Status derivation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeriveStatusRequest {
    /// "dead" → "1", "missing" → "9", anything else → no synthetic rows.
    pub disappeared_treatment: String,
}

#[derive(Debug, Serialize)]
pub struct DeriveStatusResult {
    pub summary:        DeriveStatusSummary,
    pub row_count:      usize,
    pub mapped_columns: Vec<String>,
}

#[command]
pub async fn derive_status(
    state: tauri::State<'_, AppState>,
    request: DeriveStatusRequest,
) -> Result<DeriveStatusResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded")?;

    let df = session.mapped_df.as_ref().ok_or("run apply_fields_mapping first")?;

    let treatment = match request.disappeared_treatment.as_str() {
        "dead"    => Some("1"),
        "missing" => Some("9"),
        _         => None,
    };

    let (derived_df, summary) = derive_status_column(df.clone().lazy(), treatment)
        .map_err(|e| e.to_string())?;

    let census_type = session
        .mapping
        .as_ref()
        .map(|m| m.metadata.census_type)
        .unwrap_or(CensusType::Multi);
    let final_df = prepare_mapped_frame(derived_df.lazy(), census_type)
        .collect()
        .map_err(|e| e.to_string())?;

    let row_count = final_df.height();
    let mapped_columns = final_df.get_column_names().iter().map(|s| s.to_string()).collect();

    session.mapped_df = Some(final_df);
    Ok(DeriveStatusResult { summary, row_count, mapped_columns })
}

// ---------------------------------------------------------------------------
// Diagnose: treat the raw loaded file directly as GFB3 (skips mapping wizard)
// ---------------------------------------------------------------------------

#[command]
pub async fn use_raw_as_gfb3(
    state: tauri::State<'_, AppState>,
) -> Result<ApplyMappingResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded")?;

    let raw_df = session.raw_df.clone();
    let mapped_columns = raw_df.get_column_names().iter().map(|s| s.to_string()).collect();
    let row_count = raw_df.height();

    session.mapped_df = Some(raw_df);
    session.validation_report = None;
    session.diagnostic_report = None;
    Ok(ApplyMappingResult { mapped_columns, row_count })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_census_type(metadata: &MetadataInput) -> CensusType {
    match metadata.census_type.as_deref() {
        Some("single") | Some("s") => CensusType::Single,
        _ => CensusType::Multi,
    }
}

fn field_expr_from_input(f: &FieldExprInput) -> FieldExpr {
    match f.kind.as_str() {
        "literal" => FieldExpr::Literal {
            value:      f.value.clone().unwrap_or_default(),
            target_col: f.target_col.clone(),
        },
        "concat" => FieldExpr::Concat {
            sources:    f.sources.clone().unwrap_or_default(),
            sep:        f.sep.clone().unwrap_or_else(|| "_".into()),
            target_col: f.target_col.clone(),
            to_lower:   f.to_lower.unwrap_or(true),
            prefix:     f.prefix.clone(),
        },
        "year_from_column" => FieldExpr::YearFromColumn {
            source:     f.source.clone().unwrap_or_default(),
            target_col: f.target_col.clone(),
        },
        _ => FieldExpr::Column {
            source:     f.source.clone().unwrap_or_default(),
            target_col: f.target_col.clone(),
        },
    }
}

fn status_remaps_from_input(items: &[StatusRemapInput]) -> Vec<StatusRemap> {
    items
        .iter()
        .map(|r| StatusRemap {
            source_value: r.source_value.clone(),
            target_code:  r.target_code.clone(),
            note:         r.note.clone(),
        })
        .collect()
}

/// Left-join lookup files onto `lf` for any `kind == "lookup"` entries.
/// Each lookup entry loads a separate file and joins on `main_key` = `lookup_key`,
/// adding a Float64 column named `target_col` from `value_col` in the lookup.
fn apply_lookup_joins(
    mut lf: polars::prelude::LazyFrame,
    fields: &[FieldExprInput],
) -> Result<polars::prelude::LazyFrame, String> {
    for f in fields.iter().filter(|f| f.kind == "lookup") {
        let path     = f.lookup_path.as_deref().ok_or("lookup requires lookup_path")?;
        let main_key = f.main_key.as_deref().ok_or("lookup requires main_key")?;
        let lk_key   = f.lookup_key.as_deref().ok_or("lookup requires lookup_key")?;
        let val_col  = f.value_col.as_deref().ok_or("lookup requires value_col")?;

        let lookup_df = read_file(std::path::Path::new(path))
            .map_err(|e| format!("lookup file '{path}': {e}"))?;

        // Select only the join key and value column; rename key to avoid collision.
        let lookup_lf = lookup_df.lazy().select([
            pcol(lk_key).alias("__lk_key__"),
            pcol(val_col).cast(DataType::Float64).alias(f.target_col.as_str()),
        ]);

        lf = lf.join(
            lookup_lf,
            [pcol(main_key)],
            [pcol("__lk_key__")],
            JoinArgs::new(JoinType::Left),
        );
    }
    Ok(lf)
}

fn parse_gfb3_field(s: &str) -> Option<Gfb3Field> {
    match s {
        "PlotId"  => Some(Gfb3Field::PlotId),
        "TreeId"  => Some(Gfb3Field::TreeId),
        "Yr"      => Some(Gfb3Field::Yr),
        "PrevYr"  => Some(Gfb3Field::PrevYr),
        "Status"  => Some(Gfb3Field::Status),
        "Dbh"     => Some(Gfb3Field::Dbh),
        "Species" => Some(Gfb3Field::Species),
        "Dsn"     => Some(Gfb3Field::Dsn),
        _         => None,
    }
}

// ---------------------------------------------------------------------------
// Species / TNRS
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SpeciesResolveResponse {
    pub results: Vec<TnrsResultRow>,
    pub skipped: bool,
    pub message: Option<String>,
}

/// Collect unique Species values, flag near-duplicates, and resolve via TNRS.
#[command]
pub async fn resolve_species_tnrs(
    state: tauri::State<'_, AppState>,
) -> Result<SpeciesResolveResponse, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("no file loaded")?;
    let df = session
        .mapped_df
        .as_ref()
        .ok_or("apply field mapping before species resolution")?;

    if !df.get_column_names().iter().any(|c| c.as_str() == "Species") {
        return Ok(SpeciesResolveResponse {
            results: vec![],
            skipped: true,
            message: Some("No Species column — nothing to resolve.".into()),
        });
    }

    let col = df.column("Species").map_err(|e| e.to_string())?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for i in 0..col.len() {
        let val = col.get(i).unwrap_or(AnyValue::Null);
        if matches!(val, AnyValue::Null) {
            continue;
        }
        let s = val.str_value().into_owned();
        if s.trim().is_empty() || s == "null" {
            continue;
        }
        *counts.entry(s).or_insert(0) += 1;
    }

    if counts.is_empty() {
        return Ok(SpeciesResolveResponse {
            results: vec![],
            skipped: true,
            message: Some("Species column is empty.".into()),
        });
    }

    let entries = build_species_entries(&counts);
    // Near-duplicate-only preview if TNRS is unreachable
    let body = build_tnrs_request(&entries);
    let body_json = serde_json::to_string(&body).map_err(|e| e.to_string())?;

    let resp = ureq::post(tnrs_url())
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .set("charset", "UTF-8")
        .send_string(&body_json);

    match resp {
        Ok(r) => {
            let raw: serde_json::Value = r.into_json().map_err(|e| format!("TNRS JSON: {e}"))?;
            let results = parse_tnrs_response(&raw, &entries)?;
            Ok(SpeciesResolveResponse {
                results,
                skipped: false,
                message: None,
            })
        }
        Err(e) => {
            // Offline fallback: still return near-duplicate highlights
            let results: Vec<TnrsResultRow> = entries
                .iter()
                .map(|e| TnrsResultRow {
                    original: e.original.clone(),
                    matches: vec![],
                    best_accepted: None,
                    ambiguous: !e.near_duplicates.is_empty(),
                    near_duplicates: e.near_duplicates.clone(),
                })
                .collect();
            Ok(SpeciesResolveResponse {
                results,
                skipped: false,
                message: Some(format!(
                    "TNRS unreachable ({e}). Near-duplicate highlights are still shown — pick names manually or retry."
                )),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SpeciesRemapInput {
    pub original: String,
    pub resolved: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplySpeciesRequest {
    pub remaps: Vec<SpeciesRemapInput>,
}

/// Apply chosen species resolutions onto the mapped DataFrame.
#[command]
pub async fn apply_species_resolutions(
    state: tauri::State<'_, AppState>,
    request: ApplySpeciesRequest,
) -> Result<ApplyMappingResult, String> {
    let mut guard = state.session.lock().unwrap();
    let session = guard.as_mut().ok_or("no file loaded")?;
    let df = session
        .mapped_df
        .as_ref()
        .ok_or("no mapped data")?
        .clone();

    if request.remaps.is_empty() {
        let mapped_columns = df.get_column_names().iter().map(|s| s.to_string()).collect();
        let row_count = df.height();
        return Ok(ApplyMappingResult {
            mapped_columns,
            row_count,
        });
    }

    use polars::prelude::{col, lit, when};
    let mut expr = col("Species").cast(DataType::String);
    for r in &request.remaps {
        if r.original == r.resolved {
            continue;
        }
        expr = when(col("Species").cast(DataType::String).eq(lit(r.original.clone())))
            .then(lit(r.resolved.clone()))
            .otherwise(expr);
    }
    let mapped_df = df
        .lazy()
        .with_columns([expr.alias("Species")])
        .collect()
        .map_err(|e| e.to_string())?;
    let mapped_columns = mapped_df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let row_count = mapped_df.height();
    session.mapped_df = Some(mapped_df);
    Ok(ApplyMappingResult {
        mapped_columns,
        row_count,
    })
}

// ---------------------------------------------------------------------------
// Map tab: extract plot / tree coordinates from the loaded dataframe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct MapPoint {
    pub lat: f64,
    pub lon: f64,
    pub label: Option<String>,
    pub plot_id: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MapPointsResult {
    pub points: Vec<MapPoint>,
    pub total_rows_scanned: usize,
    pub truncated: bool,
}

/// Thin a point cloud to `limit` by round-robin sampling across a geographic grid.
/// Prefer this over "first N in file order" so later / distant plots still appear.
fn subsample_spatially(points: Vec<MapPoint>, limit: usize) -> Vec<MapPoint> {
    if points.len() <= limit {
        return points;
    }

    let mut lat_min = f64::INFINITY;
    let mut lat_max = f64::NEG_INFINITY;
    let mut lon_min = f64::INFINITY;
    let mut lon_max = f64::NEG_INFINITY;
    for p in &points {
        lat_min = lat_min.min(p.lat);
        lat_max = lat_max.max(p.lat);
        lon_min = lon_min.min(p.lon);
        lon_max = lon_max.max(p.lon);
    }
    let lat_span = (lat_max - lat_min).max(1e-9);
    let lon_span = (lon_max - lon_min).max(1e-9);
    let side = ((limit as f64).sqrt().ceil() as usize).max(1);

    let mut buckets: std::collections::HashMap<(usize, usize), Vec<MapPoint>> =
        std::collections::HashMap::new();
    for p in points {
        let row = (((p.lat - lat_min) / lat_span) * side as f64).floor() as usize;
        let col = (((p.lon - lon_min) / lon_span) * side as f64).floor() as usize;
        buckets
            .entry((row.min(side - 1), col.min(side - 1)))
            .or_default()
            .push(p);
    }

    // Larger cells first so dense regions don't starve sparse ones on early rounds.
    let mut bucket_list: Vec<Vec<MapPoint>> = buckets.into_values().collect();
    bucket_list.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut out = Vec::with_capacity(limit);
    let mut depth = 0usize;
    while out.len() < limit {
        let mut progressed = false;
        for bucket in &bucket_list {
            if depth < bucket.len() {
                out.push(bucket[depth].clone());
                progressed = true;
                if out.len() >= limit {
                    break;
                }
            }
        }
        if !progressed {
            break;
        }
        depth += 1;
    }
    out
}

fn anyvalue_to_f64(v: &AnyValue) -> Option<f64> {
    match v {
        AnyValue::Null => None,
        AnyValue::Float64(x) => Some(*x),
        AnyValue::Float32(x) => Some(f64::from(*x)),
        AnyValue::Int64(x) => Some(*x as f64),
        AnyValue::Int32(x) => Some(f64::from(*x)),
        AnyValue::Int16(x) => Some(f64::from(*x)),
        AnyValue::Int8(x) => Some(f64::from(*x)),
        AnyValue::UInt64(x) => Some(*x as f64),
        AnyValue::UInt32(x) => Some(f64::from(*x)),
        AnyValue::UInt16(x) => Some(f64::from(*x)),
        AnyValue::UInt8(x) => Some(f64::from(*x)),
        AnyValue::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        AnyValue::StringOwned(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

fn anyvalue_to_label(v: &AnyValue) -> Option<String> {
    match v {
        AnyValue::Null => None,
        AnyValue::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        AnyValue::StringOwned(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        other => Some(format!("{other}")),
    }
}

#[tauri::command]
pub async fn get_map_points(
    state: tauri::State<'_, AppState>,
    lat_col: String,
    lon_col: String,
    label_col: Option<String>,
    symbol_col: Option<String>,
    max_points: Option<usize>,
) -> Result<MapPointsResult, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("No file loaded. Load a dataset first.")?;
    let df = &session.raw_df;

    let limit = max_points.unwrap_or(25_000).min(100_000);
    // Collect more unique locations than we display so spatial thinning can
    // draw from the whole file, not only the first rows.
    let collect_cap = (limit.saturating_mul(4)).max(limit).min(100_000);
    let n = df.height();
    let lat_s = df.column(&lat_col).map_err(|_| {
        format!("Latitude column “{lat_col}” not found in the loaded file.")
    })?;
    let lon_s = df.column(&lon_col).map_err(|_| {
        format!("Longitude column “{lon_col}” not found in the loaded file.")
    })?;
    let label_s = match label_col.as_deref() {
        Some(c) if !c.is_empty() => Some(df.column(c).map_err(|_| {
            format!("Label column “{c}” not found in the loaded file.")
        })?),
        _ => None,
    };
    let symbol_s = match symbol_col.as_deref() {
        Some(c) if !c.is_empty() => Some(df.column(c).map_err(|_| {
            format!("Symbol column “{c}” not found in the loaded file.")
        })?),
        _ => None,
    };
    let plot_s = df.column("PlotID").ok();

    let mut seen = std::collections::HashSet::new();
    let mut points = Vec::new();
    let mut hit_collect_cap = false;

    for i in 0..n {
        let lat = anyvalue_to_f64(&lat_s.get(i).map_err(|e| e.to_string())?);
        let lon = anyvalue_to_f64(&lon_s.get(i).map_err(|e| e.to_string())?);
        let (Some(lat), Some(lon)) = (lat, lon) else {
            continue;
        };
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        // Deduplicate at ~1e-6 deg (~10 cm) to keep the map readable
        let key = ((lat * 1e6).round() as i64, (lon * 1e6).round() as i64);
        if !seen.insert(key) {
            continue;
        }
        if points.len() >= collect_cap {
            hit_collect_cap = true;
            break;
        }
        let plot_id = plot_s
            .as_ref()
            .and_then(|s| s.get(i).ok())
            .and_then(|v| anyvalue_to_label(&v));
        let label = label_s
            .as_ref()
            .and_then(|s| s.get(i).ok())
            .and_then(|v| anyvalue_to_label(&v))
            .or_else(|| plot_id.clone());
        let symbol = symbol_s
            .as_ref()
            .and_then(|s| s.get(i).ok())
            .and_then(|v| anyvalue_to_label(&v));
        points.push(MapPoint {
            lat,
            lon,
            label,
            plot_id,
            symbol,
        });
    }

    let before = points.len();
    let points = subsample_spatially(points, limit);
    let truncated = hit_collect_cap || before > points.len();

    Ok(MapPointsResult {
        points,
        total_rows_scanned: n,
        truncated,
    })
}

#[tauri::command]
pub async fn save_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents.as_bytes()).map_err(|e| e.to_string())
}
