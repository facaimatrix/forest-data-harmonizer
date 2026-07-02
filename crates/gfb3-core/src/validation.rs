use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mapping::CensusType;
use crate::schema::{STATUS_ALIVE, STATUS_DEAD, STATUS_MISSING, STATUS_RECRUIT};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Duplicate (PlotID, TreeID, YR) — must be resolved before any lag.
    DuplicateTreeWithinPlotYear,
    /// Status value not in {"0","1","2","9"} — remapped to "9" unless the
    /// contributor provides an explicit mapping.
    UnknownStatus,
    /// Dead tree (Status == "1") has a non-null DBH — should be null.
    DeadTreeHasDbh,
    /// Recruit (Status == "2") has no DBH — row is dropped at export.
    RecruitMissingDbh,
    /// PrevYR is null AND Status == "1" — orphan dead row, dropped at export.
    OrphanDeadFirstCensus,
    /// Recruit (Status == "2") appears at the minimum YR observed for that
    /// tree — likely a mis-classified anchor or data error.
    RecruitAtMinYear,
    /// No persistent TreeID linkage across censuses — paired-census structure
    /// cannot be verified; escalate to curator.
    NoPersistentTreeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Row(s) will be silently dropped during export per GFB3 rules.
    AutoDrop,
    /// Contributor should review; default recoding will be applied (e.g. → "9").
    AutoRecode,
    /// Contributor must resolve before export can proceed.
    RequiresInput,
    /// PI-level judgment required; write to curation log, do not resolve in-app.
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedAction {
    DropRows,
    RecodeToMissing,
    NullifyDbh,
    ContributorMapping,
    EscalateToCurationLog,
    ReviewAndConfirm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFinding {
    pub rule: ValidationRule,
    pub severity: Severity,
    /// Plain-language explanation for the contributor.
    pub message: String,
    /// Number of affected rows.
    pub row_count: usize,
    pub action: RecommendedAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub findings: Vec<ValidationFinding>,
}

impl ValidationReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn has_blocking(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f.severity, Severity::RequiresInput | Severity::Escalate))
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Polars error during validation: {0}")]
    Polars(#[from] PolarsError),
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidateOptions {
    pub census_type: CensusType,
}

/// Run all GFB3 validation checks against a mapped, pre-pivot LazyFrame.
///
/// The LazyFrame must already have canonical GFB3 column names (PlotID, TreeID,
/// YR, Status, DBH, gfb3_dsn). Multi-census data must also have PrevYR.
/// Status must be a String column.
pub fn validate(lf: LazyFrame, options: ValidateOptions) -> Result<ValidationReport, ValidationError> {
    let mut findings = Vec::new();

    // Collect once for checks that need a materialized frame.
    let df = lf.clone().collect()?;

    findings.extend(check_duplicates(&df)?);
    findings.extend(check_unknown_status(&df)?);
    findings.extend(check_dead_tree_dbh(&df)?);
    findings.extend(check_recruit_missing_dbh(&df)?);
    if options.census_type != CensusType::Single {
        findings.extend(check_orphan_dead_first_census(&df)?);
        findings.extend(check_recruit_at_min_year(lf.clone())?);
    }

    Ok(ValidationReport { findings })
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// (PlotID, TreeID, YR) must be unique — dedup is required before any lag
/// computation.
fn check_duplicates(df: &DataFrame) -> Result<Vec<ValidationFinding>, ValidationError> {
    let dupes = df
        .clone()
        .lazy()
        .group_by([col("PlotID"), col("TreeID"), col("YR")])
        .agg([col("PlotID").count().alias("n")])
        .filter(col("n").gt(lit(1u32)))
        .collect()?;

    if dupes.height() == 0 {
        return Ok(vec![]);
    }

    Ok(vec![ValidationFinding {
        rule: ValidationRule::DuplicateTreeWithinPlotYear,
        severity: Severity::RequiresInput,
        message: format!(
            "{} duplicate (PlotID, TreeID, YR) combinations found. \
             Deduplication must happen before any lag or previous-value \
             computation. Review these rows and choose which to keep.",
            dupes.height()
        ),
        row_count: dupes.height(),
        action: RecommendedAction::ReviewAndConfirm,
    }])
}

/// Status values outside {"0","1","2","9"} are ambiguous and will be recoded
/// to "9" (missing) unless the contributor provides an explicit mapping.
fn check_unknown_status(df: &DataFrame) -> Result<Vec<ValidationFinding>, ValidationError> {
    let known = [
        lit(STATUS_ALIVE),
        lit(STATUS_DEAD),
        lit(STATUS_RECRUIT),
        lit(STATUS_MISSING),
    ];

    // Build: col("Status").is_in(Series::new(...)) — use filter instead
    let unknown = df
        .clone()
        .lazy()
        .filter(
            col("Status")
                .eq(lit(STATUS_ALIVE))
                .or(col("Status").eq(lit(STATUS_DEAD)))
                .or(col("Status").eq(lit(STATUS_RECRUIT)))
                .or(col("Status").eq(lit(STATUS_MISSING)))
                .not(),
        )
        .collect()?;

    let _ = known; // referenced above for clarity

    if unknown.height() == 0 {
        return Ok(vec![]);
    }

    // Surface the distinct unknown values so the contributor knows what to map.
    let distinct_values: Vec<String> = unknown
        .column("Status")?
        .str()?
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(|s| format!("{:?}", s))
        .collect();

    Ok(vec![ValidationFinding {
        rule: ValidationRule::UnknownStatus,
        severity: Severity::AutoRecode,
        message: format!(
            "{} rows have status values not in the GFB3 vocabulary \
             (found: {}). These will be recoded to \"9\" (missing) unless \
             you provide an explicit mapping in the status-vocabulary step.",
            unknown.height(),
            distinct_values.join(", ")
        ),
        row_count: unknown.height(),
        action: RecommendedAction::RecodeToMissing,
    }])
}

/// Dead trees (Status == "1") should have DBH = null.  A non-null DBH on a
/// dead tree is either a data error or a living-tree mis-classification.
fn check_dead_tree_dbh(df: &DataFrame) -> Result<Vec<ValidationFinding>, ValidationError> {
    let bad = df
        .clone()
        .lazy()
        .filter(
            col("Status")
                .eq(lit(STATUS_DEAD))
                .and(col("DBH").is_not_null()),
        )
        .collect()?;

    if bad.height() == 0 {
        return Ok(vec![]);
    }

    Ok(vec![ValidationFinding {
        rule: ValidationRule::DeadTreeHasDbh,
        severity: Severity::AutoRecode,
        message: format!(
            "{} dead trees (Status=\"1\") have a non-null DBH value. \
             Per GFB3 convention dead trees must have DBH = null. \
             These DBH values will be set to null during export.",
            bad.height()
        ),
        row_count: bad.height(),
        action: RecommendedAction::NullifyDbh,
    }])
}

/// Recruits (Status == "2") with no DBH cannot be included — dropped at export.
fn check_recruit_missing_dbh(df: &DataFrame) -> Result<Vec<ValidationFinding>, ValidationError> {
    let bad = df
        .clone()
        .lazy()
        .filter(
            col("Status")
                .eq(lit(STATUS_RECRUIT))
                .and(col("DBH").is_null()),
        )
        .collect()?;

    if bad.height() == 0 {
        return Ok(vec![]);
    }

    Ok(vec![ValidationFinding {
        rule: ValidationRule::RecruitMissingDbh,
        severity: Severity::AutoDrop,
        message: format!(
            "{} recruit rows (Status=\"2\") have no DBH measurement. \
             These rows will be dropped at export per GFB3 rules.",
            bad.height()
        ),
        row_count: bad.height(),
        action: RecommendedAction::DropRows,
    }])
}

/// Rows where PrevYR is null AND Status == "1" are dead trees with no prior
/// census link — structurally impossible in a paired-census and are dropped.
fn check_orphan_dead_first_census(
    df: &DataFrame,
) -> Result<Vec<ValidationFinding>, ValidationError> {
    let bad = df
        .clone()
        .lazy()
        .filter(
            col("PrevYR")
                .is_null()
                .and(col("Status").eq(lit(STATUS_DEAD))),
        )
        .collect()?;

    if bad.height() == 0 {
        return Ok(vec![]);
    }

    Ok(vec![ValidationFinding {
        rule: ValidationRule::OrphanDeadFirstCensus,
        severity: Severity::AutoDrop,
        message: format!(
            "{} rows have PrevYR=null AND Status=\"1\" (dead with no prior \
             census). These are structurally invalid in a paired-census dataset \
             and will be dropped at export.",
            bad.height()
        ),
        row_count: bad.height(),
        action: RecommendedAction::DropRows,
    }])
}

/// Recruits (Status == "2") must not appear at the minimum observed YR for
/// their tree — that indicates a potential anchor mis-classification.
/// Recruits are identified by YR != min(YR) per (PlotID, TreeID).
fn check_recruit_at_min_year(lf: LazyFrame) -> Result<Vec<ValidationFinding>, ValidationError> {
    let suspicious = lf
        .with_columns([col("YR")
            .min()
            .over([col("PlotID"), col("TreeID")])
            .alias("_min_yr")])
        .filter(
            col("Status")
                .eq(lit(STATUS_RECRUIT))
                .and(col("YR").eq(col("_min_yr"))),
        )
        .select([col("PlotID"), col("TreeID"), col("YR"), col("Status")])
        .collect()?;

    if suspicious.height() == 0 {
        return Ok(vec![]);
    }

    Ok(vec![ValidationFinding {
        rule: ValidationRule::RecruitAtMinYear,
        severity: Severity::Escalate,
        message: format!(
            "{} trees are marked as recruits (Status=\"2\") at the earliest \
             observed census year for that tree. Recruits are defined by \
             YR != min(YR) per tree; a recruit at min(YR) is likely an anchor \
             (Status=\"0\") or a data error. PI-level judgment required — \
             these are written to the curation log.",
            suspicious.height()
        ),
        row_count: suspicious.height(),
        action: RecommendedAction::EscalateToCurationLog,
    }])
}

// ---------------------------------------------------------------------------
// Export-time transformations (applied unconditionally, not interactive)
// ---------------------------------------------------------------------------

/// Drop all first-census anchor rows (PrevYR null AND Status == "0").
/// Must be called after all interactive validation steps are resolved.
pub fn drop_anchor_rows(lf: LazyFrame) -> LazyFrame {
    lf.filter(
        col("PrevYR")
            .is_null()
            .and(col("Status").eq(lit(STATUS_ALIVE)))
            .not(),
    )
}

/// Drop rows that are invalid per GFB3 rules (orphan dead + recruitless DBH).
/// These are the AutoDrop findings made concrete.
pub fn drop_invalid_rows(lf: LazyFrame, census_type: CensusType) -> LazyFrame {
    let lf = if census_type == CensusType::Single {
        lf
    } else {
        // Drop: PrevYR null AND Status == "1"
        lf.filter(
            col("PrevYR")
                .is_null()
                .and(col("Status").eq(lit(STATUS_DEAD)))
                .not(),
        )
    };
    // Drop: Status == "2" AND DBH null
    lf.filter(
        col("Status")
            .eq(lit(STATUS_RECRUIT))
            .and(col("DBH").is_null())
            .not(),
    )
}

/// Nullify DBH on all dead trees.
pub fn nullify_dead_dbh(lf: LazyFrame) -> LazyFrame {
    lf.with_columns([when(col("Status").eq(lit(STATUS_DEAD)))
        .then(lit(NULL).cast(DataType::Float64))
        .otherwise(col("DBH"))
        .alias("DBH")])
}

/// Recode any Status value outside the known vocabulary to "9" (missing).
pub fn recode_unknown_status(lf: LazyFrame) -> LazyFrame {
    lf.with_columns([when(
        col("Status")
            .eq(lit(STATUS_ALIVE))
            .or(col("Status").eq(lit(STATUS_DEAD)))
            .or(col("Status").eq(lit(STATUS_RECRUIT)))
            .or(col("Status").eq(lit(STATUS_MISSING))),
    )
    .then(col("Status"))
    .otherwise(lit(STATUS_MISSING))
    .alias("Status")])
}

/// Sort by (PlotID, TreeID, YR) — required before any lag / previous-value
/// computation.  Always sort before grouping.
pub fn sort_for_lag(lf: LazyFrame) -> LazyFrame {
    lf.sort(
        ["PlotID", "TreeID", "YR"],
        SortMultipleOptions::default().with_order_descending_multi([false, false, false]),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    fn make_df(
        plot: &[&str],
        tree: &[&str],
        yr: &[u32],
        prev_yr: &[Option<u32>],
        status: &[&str],
        dbh: &[Option<f64>],
    ) -> DataFrame {
        DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), plot)),
            Column::from(Series::new("TreeID".into(), tree)),
            Column::from(Series::new("YR".into(), yr)),
            Column::from(Series::new("PrevYR".into(), prev_yr)),
            Column::from(Series::new("Status".into(), status)),
            Column::from(Series::new("DBH".into(), dbh)),
        ])
        .unwrap()
    }

    #[test]
    fn no_findings_on_clean_data() {
        let df = make_df(
            &["P1", "P1"],
            &["T1", "T1"],
            &[2010, 2015],
            &[None, Some(2010)],
            &["0", "0"],
            &[Some(10.0), Some(11.0)],
        );
        let report = validate(df.lazy(), ValidateOptions::default()).unwrap();
        assert!(report.is_clean(), "{:?}", report.findings);
    }

    #[test]
    fn detects_dead_tree_with_dbh() {
        let df = make_df(
            &["P1"],
            &["T1"],
            &[2015],
            &[Some(2010)],
            &["1"],
            &[Some(10.0)], // should be null for dead tree
        );
        let report = validate(df.lazy(), ValidateOptions::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == ValidationRule::DeadTreeHasDbh));
    }

    #[test]
    fn detects_recruit_missing_dbh() {
        let df = make_df(
            &["P1"],
            &["T1"],
            &[2015],
            &[None],
            &["2"],
            &[None],
        );
        let report = validate(df.lazy(), ValidateOptions::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == ValidationRule::RecruitMissingDbh));
    }

    #[test]
    fn detects_orphan_dead_first_census() {
        let df = make_df(
            &["P1"],
            &["T1"],
            &[2010],
            &[None],
            &["1"],
            &[None],
        );
        let report = validate(df.lazy(), ValidateOptions::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == ValidationRule::OrphanDeadFirstCensus));
    }

    #[test]
    fn detects_unknown_status() {
        let df = make_df(
            &["P1"],
            &["T1"],
            &[2015],
            &[Some(2010)],
            &["alive"], // not a GFB3 status code
            &[Some(12.0)],
        );
        let report = validate(df.lazy(), ValidateOptions::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == ValidationRule::UnknownStatus));
    }

    #[test]
    fn single_census_validates_without_prevyr() {
        let df = DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1"])),
            Column::from(Series::new("TreeID".into(), &["T1"])),
            Column::from(Series::new("YR".into(), &[2015u32])),
            Column::from(Series::new("Status".into(), &["0"])),
            Column::from(Series::new("DBH".into(), &[Some(10.0f64)])),
            Column::from(Series::new("gfb3_dsn".into(), &["in_test"])),
        ])
        .unwrap();
        let report = validate(
            df.lazy(),
            ValidateOptions {
                census_type: CensusType::Single,
            },
        )
        .unwrap();
        assert!(report.is_clean(), "{:?}", report.findings);
    }

    #[test]
    fn drop_anchor_rows_removes_correct_rows() {
        let df = make_df(
            &["P1", "P1", "P1"],
            &["T1", "T1", "T2"],
            &[2010, 2015, 2010],
            &[None, Some(2010), None],
            &["0", "0", "0"],
            &[Some(10.0), Some(11.0), Some(8.0)],
        );
        let result = drop_anchor_rows(df.lazy()).collect().unwrap();
        // Only the 2015 row survives (PrevYR not null)
        assert_eq!(result.height(), 1);
    }

    #[test]
    fn recode_unknown_status_maps_to_nine() {
        let df = DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1"])),
            Column::from(Series::new("TreeID".into(), &["T1"])),
            Column::from(Series::new("YR".into(), &[2015u32])),
            Column::from(Series::new("PrevYR".into(), &[Some(2010u32)])),
            Column::from(Series::new("Status".into(), &["n/a"])),
            Column::from(Series::new("DBH".into(), &[Some(10.0f64)])),
        ])
        .unwrap();
        let result = recode_unknown_status(df.lazy()).collect().unwrap();
        let status = result.column("Status").unwrap().str().unwrap();
        assert_eq!(status.get(0), Some(STATUS_MISSING));
    }
}
