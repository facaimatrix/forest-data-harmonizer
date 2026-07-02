use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::command;

use gfb3_core::export::{coerce_status_to_int, draft_filename, write_csv, write_parquet, write_xlsx, Provenance};
use gfb3_core::log::CurationLog;
use gfb3_core::mapping::{
    CensusType, ColumnMapping, ContributorMapping, DatasetMetadata, DbhUnit, StatusRemap,
};
use gfb3_core::reader::{dataframe_preview, read_file};
use gfb3_core::schema::{gfb3_field_defs, Gfb3Field, InputGate};
use gfb3_core::transform::{
    apply_column_mapping, apply_field_exprs, apply_status_remap, derive_status_column,
    melt_wide_to_long, prepare_mapped_frame, scale_dbh_mm_to_cm, DeriveStatusSummary, FieldExpr,
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
    /// First 10 rows, row-major, values as strings (nulls → null in JSON).
    pub preview_rows: Vec<Vec<Option<String>>>,
    /// Plain-language structural gate errors (empty = passed).
    pub gate_errors: Vec<String>,
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

    let gate_errors: Vec<String> = InputGate::check(&df)
        .into_iter()
        .map(|e| e.to_string())
        .collect();

    let columns: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let suggestions = ContributorMapping::suggest_from_headers(&columns);
    let suggested_mappings = columns
        .iter()
        .map(|col| {
            let field = suggestions
                .iter()
                .find(|m| &m.source_column == col)
                .map(|m| format!("{:?}", m.target_field));
            SuggestedMapping {
                source_column: col.clone(),
                suggested_gfb3_field: field,
            }
        })
        .collect();

    let preview_rows = dataframe_preview(&df, 10);
    let row_count = df.height();

    *state.session.lock().unwrap() = Some(SessionState {
        raw_df: df,
        file_path: path,
        mapped_df: None,
        mapping: None,
        validation_report: None,
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
    /// "cm" or "mm"
    pub dbh_unit: Option<String>,
    pub census_years: Vec<u32>,
    /// "single" or "multi" (defaults to multi)
    pub census_type: Option<String>,
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
    let metadata = DatasetMetadata {
        country: request.metadata.country,
        site: request.metadata.site,
        pi: request.metadata.pi,
        dbh_unit,
        coordinate_crs: None,
        census_years: request.metadata.census_years,
        census_type,
    };

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
/// show the status-vocabulary editor (step 4).
#[command]
pub async fn get_status_vocab(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StatusVocabRow>, String> {
    let guard = state.session.lock().unwrap();
    let session = guard.as_ref().ok_or("no file loaded")?;

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
        if matches!(val, AnyValue::Null) {
            continue;
        }
        *counts.entry(val.to_string()).or_insert(0) += 1;
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
// Step 5: Validation
// ---------------------------------------------------------------------------

#[command]
pub async fn run_validation(
    state: tauri::State<'_, AppState>,
) -> Result<ValidationReport, String> {
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
        .unwrap_or(CensusType::Multi);

    let report = validate(
        df.clone().lazy(),
        ValidateOptions { census_type },
    )
    .map_err(|e| e.to_string())?;
    session.validation_report = Some(report.clone());
    Ok(report)
}

// ---------------------------------------------------------------------------
// Step 6: Export
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub output_dir: String,
    pub base_name: String,
    pub formats: Vec<String>,
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

    // Full export pipeline: recode → nullify → drop invalids → drop anchors
    //                        → sort for lag → coerce status to Int32
    let lf = mapped_df.clone().lazy();
    let lf = recode_unknown_status(lf);
    let lf = nullify_dead_dbh(lf);
    let lf = drop_invalid_rows(lf, census_type);
    let lf = if census_type == CensusType::Single {
        lf
    } else {
        drop_anchor_rows(lf)
    };
    let lf = sort_for_lag(lf);
    let lf = coerce_status_to_int(lf);
    let export_df = lf.collect().map_err(|e| e.to_string())?;

    let provenance = Provenance::new_draft(dsn);
    let out_dir = std::path::Path::new(&request.output_dir);
    let mut written: Vec<String> = Vec::new();

    for fmt in &request.formats {
        let path = out_dir.join(draft_filename(&request.base_name, fmt));
        let result = match fmt.as_str() {
            "csv"     => write_csv(export_df.clone(), &path, &provenance),
            "parquet" => write_parquet(export_df.clone(), &path, &provenance),
            "xlsx"    => write_xlsx(export_df.clone(), &path, &provenance),
            other     => return Err(format!("unknown format '{other}'")),
        };
        result.map_err(|e| format!("{fmt} export failed: {e}"))?;
        written.push(path.to_string_lossy().into_owned());
    }

    // Curation log skeleton
    let mut log = CurationLog::new("Francisco Rivas");
    if let Some(mapping) = &session.mapping {
        log.prefill_from_metadata(&mapping.metadata, &mapping.gfb3_dsn);
    }
    if let Some(report) = &session.validation_report {
        log.append_escalated_findings(&report.findings);
    }
    let log_path = out_dir.join(format!("{}_curation_log.txt", request.base_name));
    std::fs::write(&log_path, log.render()).map_err(|e| e.to_string())?;
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
        .map(|f| match f.kind.as_str() {
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
        _ => FieldExpr::Column {
            source:     f.source.clone().unwrap_or_default(),
            target_col: f.target_col.clone(),
        },
    }).collect();

    let lf = apply_field_exprs(lf, &exprs, &request.gfb3_dsn);
    let renamed_df = lf.collect().map_err(|e| e.to_string())?;

    // id_cols = target column names from the exprs + gfb3_dsn.
    let mut id_cols: Vec<String> = exprs.iter().map(|e| match e {
        FieldExpr::Column  { target_col, .. } => target_col.clone(),
        FieldExpr::Literal { target_col, .. } => target_col.clone(),
        FieldExpr::Concat  { target_col, .. } => target_col.clone(),
    }).collect();
    id_cols.push("gfb3_dsn".to_string());

    let id_refs: Vec<&str> = id_cols.iter().map(|s| s.as_str()).collect();
    let pairs_ref: Vec<(&str, u32)> = request
        .dbh_pairs
        .iter()
        .map(|p| (p.source_column.as_str(), p.year))
        .collect();

    let long_df = melt_wide_to_long(&renamed_df, &id_refs, &pairs_ref)
        .map_err(|e| e.to_string())?;

    let status_remaps: Vec<StatusRemap> = request
        .status_remaps
        .into_iter()
        .map(|r| StatusRemap { source_value: r.source_value, target_code: r.target_code, note: r.note })
        .collect();
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

    let metadata = DatasetMetadata {
        country: request.metadata.country,
        site: request.metadata.site,
        pi: request.metadata.pi,
        dbh_unit,
        coordinate_crs: None,
        census_years: request.metadata.census_years,
        census_type,
    };
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
        .map(|f| match f.kind.as_str() {
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
        _ => FieldExpr::Column {
            source:     f.source.clone().unwrap_or_default(),
            target_col: f.target_col.clone(),
        },
    }).collect();

    let lf = apply_field_exprs(lf, &exprs, &request.gfb3_dsn);

    let remap_pairs: Vec<(String, String)> = request.status_remaps.iter()
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

    let metadata = DatasetMetadata {
        country:        request.metadata.country,
        site:           request.metadata.site,
        pi:             request.metadata.pi,
        dbh_unit:       dbh_unit_enum,
        coordinate_crs: None,
        census_years:   request.metadata.census_years,
        census_type,
    };
    let mapping = ContributorMapping {
        gfb3_dsn:        request.gfb3_dsn,
        column_mappings: vec![],
        status_remaps:   vec![],
        needs_pivot:     false,
        wide_dbh_columns: vec![],
        metadata,
    };

    session.mapped_df = Some(mapped_df);
    session.mapping = Some(mapping);
    session.validation_report = None;

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
