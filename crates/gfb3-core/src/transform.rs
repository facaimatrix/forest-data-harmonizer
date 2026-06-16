use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mapping::ColumnMapping;
use crate::schema::gfb3_field_defs;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Polars error during transform: {0}")]
    Polars(#[from] PolarsError),
    #[error("wide-to-long pivot failed: {0}")]
    Pivot(String),
    #[error("deduplication failed: no sort key columns found")]
    MissingSortKey,
}

/// Deduplicate by (PlotID, TreeID, YR), keeping the first occurrence after
/// sorting by (PlotID, TreeID, YR).  Must be called before any lag computation.
pub fn dedup(lf: LazyFrame) -> LazyFrame {
    lf.sort(
        ["PlotID", "TreeID", "YR"],
        SortMultipleOptions::default().with_order_descending_multi([false, false, false]),
    )
    .unique_stable(
        Some(vec!["PlotID".into(), "TreeID".into(), "YR".into()]),
        UniqueKeepStrategy::First,
    )
}

/// Compute the PrevYR column as the lag-1 YR per (PlotID, TreeID).
///
/// Requires the data to already be sorted by (PlotID, TreeID, YR) and
/// deduplicated.
pub fn compute_prev_yr(lf: LazyFrame) -> LazyFrame {
    lf.with_columns([col("YR")
        .shift(lit(1))
        .over([col("PlotID"), col("TreeID")])
        .alias("PrevYR")])
}

/// Melt a wide-format dataset into long format.
///
/// `id_columns`: columns shared across all censuses (e.g. PlotID, TreeID, Status, Species,
///   gfb3_dsn). These are kept on every output row.
/// `dbh_pairs`: `(source_column_name, census_year)` — each DBH column paired with its year.
///
/// Produces a long-format `DataFrame` with `id_columns` + `DBH` (f64) + `YR` (i32).
/// The caller is responsible for calling `sort_for_lag`, `dedup`, and `compute_prev_yr`
/// after this function.
pub fn melt_wide_to_long(
    df: &DataFrame,
    id_columns: &[&str],
    dbh_pairs: &[(&str, u32)],
) -> Result<DataFrame, TransformError> {
    if dbh_pairs.is_empty() {
        return Err(TransformError::Pivot("no DBH column pairs provided".into()));
    }

    let frames: Vec<DataFrame> = dbh_pairs
        .iter()
        .map(|(col_name, year)| {
            let mut exprs: Vec<Expr> = id_columns.iter().map(|c| col(*c)).collect();
            exprs.push(col(*col_name).cast(DataType::Float64).alias("DBH"));
            exprs.push(lit(*year as i32).alias("YR"));
            df.clone()
                .lazy()
                .select(exprs)
                .collect()
                .map_err(TransformError::Polars)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut result = frames[0].clone();
    for frame in &frames[1..] {
        result = result.vstack(frame).map_err(TransformError::Polars)?;
    }
    Ok(result)
}

/// Apply a dataset-specific status remap using a mapping table.
///
/// `remap`: pairs of (source_value, target_code).  Values not in the remap
/// are left unchanged (they will be caught by validation and recoded to "9"
/// if still unknown after this step).
pub fn apply_status_remap(lf: LazyFrame, remap: &[(String, String)]) -> LazyFrame {
    if remap.is_empty() {
        return lf;
    }
    // Build a chained when/then expression.
    let mut expr = col("Status");
    for (src, tgt) in remap {
        expr = when(col("Status").eq(lit(src.clone())))
            .then(lit(tgt.clone()))
            .otherwise(expr);
    }
    lf.with_columns([expr.alias("Status")])
}

/// Rename source columns to their canonical GFB3 names and add a `gfb3_dsn`
/// literal column.  Columns not present in `mappings` are kept unchanged
/// (they will be non-GFB3 extras; cast to String when binding multiple files).
///
/// Must be called before any validation or transform that expects GFB3 column
/// names.
pub fn apply_column_mapping(
    lf: LazyFrame,
    mappings: &[ColumnMapping],
    gfb3_dsn: &str,
) -> LazyFrame {
    let field_defs = gfb3_field_defs();

    let old_names: Vec<String> = mappings.iter().map(|m| m.source_column.clone()).collect();
    let new_names: Vec<String> = mappings
        .iter()
        .map(|m| {
            field_defs
                .iter()
                .find(|d| d.field == m.target_field)
                .map(|d| d.column_name.to_string())
                .unwrap_or_else(|| m.source_column.clone())
        })
        .collect();

    // `strict = false` so columns absent from the frame are silently skipped.
    lf.rename(old_names, new_names, false)
        .with_columns([lit(gfb3_dsn.to_string()).alias("gfb3_dsn")])
}

/// Scale DBH from mm to cm (divide by 10).  Only call when the source unit
/// is confirmed as mm by the contributor in step 1.
pub fn scale_dbh_mm_to_cm(lf: LazyFrame) -> LazyFrame {
    lf.with_columns([(col("DBH") / lit(10.0f64)).alias("DBH")])
}

// ── Status derivation ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DeriveStatusSummary {
    pub first_census_count: usize,
    pub recruit_count: usize,
    pub subsequent_alive_count: usize,
    pub dead_count: usize,
    pub missing_count: usize,
    pub disappeared_tree_count: usize,
}

/// Derives the Status column from tree/census structure.
///
/// Rules (CLAUDE.md):
/// - Trees whose first appearance is the global min YR → Status "0" (alive anchor)
/// - Trees first appearing in a later census → Status "2" (recruit)
/// - All subsequent censuses for any tree → Status "0" (alive)
///
/// `disappeared_treatment`: if `Some("1")` or `Some("9")`, synthetic rows are
/// appended for trees absent from the latest (global max) census year.
pub fn derive_status_column(
    lf: LazyFrame,
    disappeared_treatment: Option<&str>,
) -> Result<(DataFrame, DeriveStatusSummary), PolarsError> {
    // ── Normalise DBH ─────────────────────────────────────────────────────────
    // Many datasets store missing values as empty cells (read as "") rather than
    // explicit null. Coerce DBH to Float64, treating "", "na", "n/a", "nan",
    // "null", "nd", and "." as null so they correctly signal dead/missing trees.
    let dbh_str = col("DBH").cast(DataType::String).str().to_lowercase();
    let lf = lf.with_columns([
        when(
            col("DBH").is_null()
                .or(dbh_str.clone().eq(lit("")))
                .or(dbh_str.clone().eq(lit("na")))
                .or(dbh_str.clone().eq(lit("n/a")))
                .or(dbh_str.clone().eq(lit("nan")))
                .or(dbh_str.clone().eq(lit("null")))
                .or(dbh_str.clone().eq(lit("nd")))
                .or(dbh_str.eq(lit(".")))
        )
        .then(lit(LiteralValue::Null))
        .otherwise(col("DBH").cast(DataType::Float64))
        .alias("DBH"),
    ]);

    let lf = lf.sort(
        ["PlotID", "TreeID", "YR"],
        SortMultipleOptions::default().with_order_descending_multi([false, false, false]),
    );

    // Global min / max YR
    let yr_stats = lf.clone()
        .select([
            col("YR").cast(DataType::Int64).min().alias("mn"),
            col("YR").cast(DataType::Int64).max().alias("mx"),
        ])
        .collect()?;
    let global_min_yr = yr_stats.column("mn")?.i64()?.get(0).unwrap_or(0);
    let global_max_yr = yr_stats.column("mx")?.i64()?.get(0).unwrap_or(0);

    // Per-tree and per-plot window columns:
    //   _tmn                 = first census year this tree appears (any DBH, incl. nulls)
    //   _tmx                 = last census year this tree appears (any DBH)
    //   _plot_first_yr       = first census year of the plot — used for recruit detection
    //                          (different plots may start in different years)
    //   _tree_first_valid_yr = first census year this tree has a non-null DBH
    //                          (used for recruit check — avoids counting null rows from
    //                           wide-format melt as the tree's "first appearance")
    //   _last_valid_yr       = last census year this tree has a non-null DBH
    let lf = lf.with_columns([
        col("YR").cast(DataType::Int64)
            .min().over([col("PlotID"), col("TreeID")])
            .alias("_tmn"),
        col("YR").cast(DataType::Int64)
            .max().over([col("PlotID"), col("TreeID")])
            .alias("_tmx"),
        // Per-PLOT first census year (not global) — recruit = tree first appears after this
        col("YR").cast(DataType::Int64)
            .min().over([col("PlotID")])
            .alias("_plot_first_yr"),
        // First census year this tree has a valid (non-null) DBH — used for recruit detection
        col("YR").cast(DataType::Int64)
            .filter(col("DBH").is_not_null())
            .min()
            .over([col("PlotID"), col("TreeID")])
            .alias("_tree_first_valid_yr"),
        // Last census year this tree had a valid (non-null) DBH
        col("YR").cast(DataType::Int64)
            .filter(col("DBH").is_not_null())
            .max()
            .over([col("PlotID"), col("TreeID")])
            .alias("_last_valid_yr"),
    ]);

    // Status derivation (priority order):
    //   "2"  recruit — first row with valid DBH for this tree, and the plot was already
    //                  censused before (handles empty-cell datasets from wide-format melt)
    //   "1"  dead    — DBH null AND last valid DBH was in a strictly earlier census
    //   "9"  missing — DBH null AND will reappear with valid DBH, OR never had valid DBH
    //   "0"  alive   — DBH present, not a recruit row
    let lf = lf.with_columns([
        when(
            // Recruit: this is the tree's first census with a measured DBH, AND the plot
            // had already been censused before this tree's first valid measurement.
            col("YR").cast(DataType::Int64).eq(col("_tree_first_valid_yr"))
                .and(col("_tree_first_valid_yr").gt(col("_plot_first_yr")))
        ).then(lit("2"))
        .when(
            // Dead: DBH null AND no future valid DBH for this tree
            col("DBH").is_null()
                .and(col("_last_valid_yr").is_not_null())
                .and(col("YR").cast(DataType::Int64).gt(col("_last_valid_yr")))
        ).then(lit("1"))
        .when(col("DBH").is_null())
        // Missing: DBH null but tree had / will have a valid DBH (gap), or no valid DBH ever
        .then(lit("9"))
        .otherwise(lit("0"))
        .alias("Status"),
    ]);

    let df = lf.collect()?;

    // Summary counts
    let count_status = |s: &str| -> Result<usize, PolarsError> {
        Ok(df.clone().lazy().filter(col("Status").eq(lit(s))).collect()?.height())
    };
    let recruit_count  = count_status("2")?;
    let dead_count     = count_status("1")?;
    let missing_count  = count_status("9")?;
    // "First-census rows" = rows in their plot's own first census (alive anchors, Status 0)
    let first_census_count = df.clone().lazy()
        .filter(col("YR").cast(DataType::Int64).eq(col("_plot_first_yr")))
        .collect()?.height();
    let subsequent_alive_count = df.height()
        .saturating_sub(first_census_count + recruit_count + dead_count + missing_count);

    // Trees whose last row (any DBH) is before the global last census — truly absent from end
    let disappeared_tree_count = df.clone().lazy()
        .filter(col("_tmx").lt(lit(global_max_yr)))
        .select([col("PlotID"), col("TreeID")])
        .unique(None, UniqueKeepStrategy::First)
        .collect()?.height();

    let df = df.drop_many(["_tmn", "_tmx", "_plot_first_yr", "_tree_first_valid_yr", "_last_valid_yr"]);

    let df = match disappeared_treatment {
        Some(t) if disappeared_tree_count > 0 => add_disappeared_rows(df, global_max_yr, t)?,
        _ => df,
    };

    Ok((df, DeriveStatusSummary {
        first_census_count,
        recruit_count,
        subsequent_alive_count,
        dead_count,
        missing_count,
        disappeared_tree_count,
    }))
}

fn add_disappeared_rows(df: DataFrame, global_max_yr: i64, treatment: &str) -> Result<DataFrame, PolarsError> {
    let yr_dtype = df.column("YR")?.dtype().clone();

    // Last-known row for each tree absent from the final census
    let last_rows = df.clone().lazy()
        .with_columns([
            col("YR").cast(DataType::Int64)
                .max().over([col("PlotID"), col("TreeID")])
                .alias("_mx"),
        ])
        .filter(
            col("_mx").lt(lit(global_max_yr))
                .and(col("YR").cast(DataType::Int64).eq(col("_mx"))),
        )
        .drop(["_mx"])
        .collect()?;

    let n = last_rows.height();
    if n == 0 {
        return Ok(df);
    }

    let mut synthetic = last_rows;

    let new_yr = Series::new("YR".into(), vec![global_max_yr as u32; n]).cast(&yr_dtype)?;
    synthetic.replace("YR", new_yr)?;

    let new_status = Series::new("Status".into(), vec![treatment; n]);
    synthetic.replace("Status", new_status)?;

    let col_names: Vec<String> = synthetic.get_column_names().iter().map(|s| s.to_string()).collect();

    if col_names.contains(&"DBH".to_string()) {
        let null_dbh = Series::new("DBH".into(), vec![Option::<f64>::None; n]);
        synthetic.replace("DBH", null_dbh)?;
    }

    if col_names.contains(&"PrevYR".to_string()) {
        let prevyr_dtype = df.column("PrevYR")?.dtype().clone();
        let null_prevyr = Series::new("PrevYR".into(), vec![Option::<u32>::None; n]).cast(&prevyr_dtype)?;
        synthetic.replace("PrevYR", null_prevyr)?;
    }

    df.vstack(&synthetic)
}

// ── Extended field expressions (field wizard) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldExpr {
    Column  { source: String, target_col: String },
    Literal { value: String,  target_col: String },
    Concat  { sources: Vec<String>, sep: String, target_col: String, to_lower: bool, prefix: Option<String> },
}

/// Apply the field wizard's field expressions to the raw DataFrame.
/// Each expr produces one GFB3-named column; columns not covered by any expr
/// are left as-is (extra/non-GFB3 columns are harmless at this stage).
pub fn apply_field_exprs(lf: LazyFrame, exprs: &[FieldExpr], gfb3_dsn: &str) -> LazyFrame {
    let mut lf = lf.with_columns([lit(gfb3_dsn).alias("gfb3_dsn")]);

    for expr in exprs {
        lf = match expr {
            FieldExpr::Column { source, target_col } => {
                if source == target_col {
                    lf
                } else {
                    lf.rename([source.clone()], [target_col.clone()], false)
                }
            }
            FieldExpr::Literal { value, target_col } => {
                // Parse as f64 for numeric GFB3 fields (Latitude, Longitude, PA)
                if let Ok(f) = value.parse::<f64>() {
                    lf.with_columns([lit(f).alias(target_col.as_str())])
                } else {
                    lf.with_columns([lit(value.clone()).alias(target_col.as_str())])
                }
            }
            FieldExpr::Concat { sources, sep, target_col, to_lower, prefix } => {
                let mut col_exprs: Vec<Expr> = Vec::new();
                if let Some(pfx) = prefix {
                    if !pfx.is_empty() {
                        col_exprs.push(lit(pfx.clone()).cast(DataType::String));
                    }
                }
                col_exprs.extend(sources.iter().map(|s| col(s.as_str()).cast(DataType::String)));
                let concat_expr = polars::prelude::concat_str(col_exprs, sep.as_str(), true);
                let concat_expr = if *to_lower {
                    concat_expr.str().to_lowercase()
                } else {
                    concat_expr
                };
                lf.with_columns([concat_expr.alias(target_col.as_str())])
            }
        };
    }
    lf
}
