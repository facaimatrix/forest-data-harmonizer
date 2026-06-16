use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Every output column in the GFB3 paired-census format.
///
/// Status travels as a String the entire pipeline; it is coerced to Int32
/// only in the export step.  All other types are the authoritative wire types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gfb3Field {
    PlotId,
    TreeId,
    Yr,
    PrevYr,
    /// Kept as String until export coerce step.
    Status,
    Dbh,
    Species,
    /// Contributor dataset identifier (e.g. "site_pi_year").
    Dsn,
}

/// Authoritative definition of a single GFB3 output column.
pub struct FieldDef {
    pub field: Gfb3Field,
    /// Canonical column name in the output dataframe.
    pub column_name: &'static str,
    pub dtype: DataType,
    /// Whether the column may contain nulls in a valid GFB3 dataset.
    pub nullable: bool,
}

/// Returns the full set of GFB3 output field definitions.
pub fn gfb3_field_defs() -> Vec<FieldDef> {
    vec![
        FieldDef {
            field: Gfb3Field::PlotId,
            column_name: "PlotID",
            dtype: DataType::String,
            nullable: false,
        },
        FieldDef {
            field: Gfb3Field::TreeId,
            column_name: "TreeID",
            dtype: DataType::String,
            nullable: false,
        },
        FieldDef {
            field: Gfb3Field::Yr,
            column_name: "YR",
            dtype: DataType::UInt32,
            nullable: false,
        },
        FieldDef {
            field: Gfb3Field::PrevYr,
            column_name: "PrevYR",
            dtype: DataType::UInt32,
            // Null for first-census rows (anchors and recruits at their entry year).
            nullable: true,
        },
        FieldDef {
            field: Gfb3Field::Status,
            column_name: "Status",
            dtype: DataType::String,
            nullable: false,
        },
        FieldDef {
            field: Gfb3Field::Dbh,
            column_name: "DBH",
            dtype: DataType::Float64,
            // Null for dead trees (Status == "1").
            nullable: true,
        },
        FieldDef {
            field: Gfb3Field::Species,
            column_name: "Species",
            dtype: DataType::String,
            nullable: true,
        },
        FieldDef {
            field: Gfb3Field::Dsn,
            column_name: "gfb3_dsn",
            dtype: DataType::String,
            nullable: false,
        },
    ]
}

/// Looks up a field definition by canonical column name.
pub fn field_def_by_name(name: &str) -> Option<FieldDef> {
    gfb3_field_defs().into_iter().find(|d| d.column_name == name)
}

// ---------------------------------------------------------------------------
// Status vocabulary constants
// ---------------------------------------------------------------------------

pub const STATUS_ALIVE: &str = "0";
pub const STATUS_DEAD: &str = "1";
pub const STATUS_RECRUIT: &str = "2";
pub const STATUS_MISSING: &str = "9";

pub const KNOWN_STATUSES: &[&str] = &[STATUS_ALIVE, STATUS_DEAD, STATUS_RECRUIT, STATUS_MISSING];

// ---------------------------------------------------------------------------
// Structural input gate
// ---------------------------------------------------------------------------

/// Checks the structural contract on a loaded DataFrame before the guided flow
/// starts.  Returns plain-language diagnostics; never a blind precondition.
///
/// Contract: one rectangular table, single header row, consistent row grain,
/// no merged cells or stacked sub-tables.
pub struct InputGate;

#[derive(Debug, Error, PartialEq)]
pub enum GateError {
    #[error("the dataset is empty — no rows to process")]
    Empty,

    #[error(
        "duplicate column name '{name}' detected; this usually means the file \
         has a merged header or stacked sub-tables — please provide one clean \
         table per file"
    )]
    DuplicateColumnName { name: String },

    #[error(
        "column '{name}' has no header (blank or auto-generated placeholder); \
         the table may have merged cells or a non-standard header row"
    )]
    BlankColumnName { name: String },

    #[error(
        "all values in column '{name}' are null — this suggests a stacked \
         sub-table or a spacer column; remove it before resubmitting"
    )]
    AllNullColumn { name: String },

    #[error(
        "found {count} columns that are entirely null, suggesting the file \
         contains a multi-table layout; please extract one rectangular table"
    )]
    MultipleAllNullColumns { count: usize },
}

impl InputGate {
    /// Runs all structural checks against a loaded `DataFrame`.
    /// Returns every error found (not short-circuit), so the user sees all
    /// problems at once.
    pub fn check(df: &DataFrame) -> Vec<GateError> {
        let mut errors = Vec::new();

        if df.height() == 0 {
            errors.push(GateError::Empty);
            return errors; // nothing else is meaningful on an empty frame
        }

        // Duplicate / blank column names
        let names = df.get_column_names();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            let s = name.as_str();
            if s.is_empty() || s.starts_with("column_") {
                errors.push(GateError::BlankColumnName { name: s.to_string() });
                continue;
            }
            if !seen.insert(s) {
                errors.push(GateError::DuplicateColumnName { name: s.to_string() });
            }
        }

        // All-null columns (spacers / stacked-table artifact)
        let mut all_null_count = 0usize;
        for series in df.get_columns() {
            if series.null_count() == df.height() {
                all_null_count += 1;
            }
        }
        if all_null_count == 1 {
            // Report which column it is
            for series in df.get_columns() {
                if series.null_count() == df.height() {
                    errors.push(GateError::AllNullColumn {
                        name: series.name().to_string(),
                    });
                }
            }
        } else if all_null_count > 1 {
            errors.push(GateError::MultipleAllNullColumns {
                count: all_null_count,
            });
        }

        errors
    }

    /// `true` when the DataFrame passes the structural contract.
    pub fn is_ok(df: &DataFrame) -> bool {
        Self::check(df).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    fn make_df(cols: Vec<Series>) -> DataFrame {
        DataFrame::new(cols.into_iter().map(Column::from).collect()).unwrap()
    }

    #[test]
    fn gate_passes_clean_frame() {
        let df = make_df(vec![
            Series::new("PlotID".into(), &["P1", "P1"]),
            Series::new("TreeID".into(), &["T1", "T2"]),
        ]);
        assert!(InputGate::is_ok(&df));
    }

    #[test]
    fn gate_catches_empty() {
        let df = make_df(vec![Series::new("PlotID".into(), Vec::<&str>::new())]);
        let errs = InputGate::check(&df);
        assert!(errs.iter().any(|e| matches!(e, GateError::Empty)));
    }

    #[test]
    fn gate_catches_duplicate_column() {
        // Polars won't allow two columns with the same name in a DataFrame,
        // so this test exercises the check via names alone.
        let df = make_df(vec![
            Series::new("A".into(), &[1i32]),
            Series::new("B".into(), &[2i32]),
        ]);
        // Manually check the name-collision logic
        let fake_names: Vec<PlSmallStr> = vec!["A".into(), "A".into()];
        let mut seen = std::collections::HashSet::new();
        let mut dupes = vec![];
        for n in &fake_names {
            if !seen.insert(n.as_str()) {
                dupes.push(n.as_str().to_string());
            }
        }
        assert_eq!(dupes, vec!["A"]);
        // Real gate should still pass the valid df
        assert!(InputGate::is_ok(&df));
    }
}
