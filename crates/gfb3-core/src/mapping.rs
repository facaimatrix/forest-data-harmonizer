use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::Gfb3Field;

/// Maps a single source column to a GFB3 output field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    /// Source column name as it appears in the contributor's file.
    pub source_column: String,
    pub target_field: Gfb3Field,
}

/// Maps contributor status vocabulary to GFB3 status codes.
///
/// Values not listed here are recoded to "9" (missing) per curation policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRemap {
    /// Contributor's status value (e.g. "alive", "dead", "n/a").
    pub source_value: String,
    /// GFB3 status code ("0", "1", "2", or "9").
    pub target_code: String,
    /// Optional note preserved in the curation log (required for "1" from
    /// anthropogenic-removal cases).
    pub note: Option<String>,
}

/// Full per-contributor mapping configuration.  Saved keyed to `gfb3_dsn`
/// so repeat submissions reuse it without re-entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorMapping {
    /// Contributor dataset identifier — unique key for storage and lookup.
    pub gfb3_dsn: String,
    pub column_mappings: Vec<ColumnMapping>,
    pub status_remaps: Vec<StatusRemap>,
    /// Whether the source data is in wide format and needs a melt/pivot step.
    pub needs_pivot: bool,
    /// For wide data: which source columns represent per-census DBH measurements.
    /// The column name encodes the census year (e.g. "DBH_2010", "DBH_2015").
    pub wide_dbh_columns: Vec<String>,
    /// Declared dataset-level metadata the contributor provides in step 1.
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CensusType {
    #[default]
    Multi,
    Single,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub country: Option<String>,
    pub site: Option<String>,
    pub pi: Option<String>,
    pub dbh_unit: Option<DbhUnit>,
    pub coordinate_crs: Option<String>,
    pub census_years: Vec<u32>,
    #[serde(default)]
    pub census_type: CensusType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbhUnit {
    Cm,
    Mm,
}

impl ContributorMapping {
    /// Returns the target field for a given source column name, if mapped.
    pub fn target_for(&self, source: &str) -> Option<Gfb3Field> {
        self.column_mappings
            .iter()
            .find(|m| m.source_column == source)
            .map(|m| m.target_field)
    }

    /// Returns the GFB3 status code for a contributor status value.
    /// Falls back to "9" (missing) for unmapped values.
    pub fn remap_status<'a>(&'a self, source_value: &str) -> &'a str {
        self.status_remaps
            .iter()
            .find(|r| r.source_value == source_value)
            .map(|r| r.target_code.as_str())
            .unwrap_or("9")
    }

    /// Fuzzy-suggest column mappings from source headers.
    ///
    /// Uses simple normalised-string similarity — the contributor confirms or
    /// corrects in the UI.  Not a commitment, just a starting point.
    pub fn suggest_from_headers(headers: &[String]) -> Vec<ColumnMapping> {
        let known: &[(&str, Gfb3Field, &[&str])] = &[
            ("PlotID", Gfb3Field::PlotId, &["plot", "plotid", "plot_id", "site"]),
            ("TreeID", Gfb3Field::TreeId, &["tree", "treeid", "tree_id", "individual", "id"]),
            ("YR",     Gfb3Field::Yr,     &["yr", "year", "census_year", "survey_year"]),
            ("PrevYR", Gfb3Field::PrevYr, &["prevyr", "prev_yr", "previous_year", "prev_year"]),
            ("Status", Gfb3Field::Status, &["status", "condition", "state", "fate"]),
            ("DBH",    Gfb3Field::Dbh,    &["dbh", "diameter", "diam", "d130", "d_bh"]),
            ("Species",Gfb3Field::Species,&["species", "sp", "taxon", "spp", "scientific"]),
        ];

        let mut suggestions = Vec::new();
        let mut used_targets = std::collections::HashSet::new();

        'outer: for header in headers {
            let norm = header.to_lowercase().replace([' ', '-'], "_");
            for (_, field, aliases) in known {
                if used_targets.contains(field) {
                    continue;
                }
                if aliases.iter().any(|a| norm.contains(a)) {
                    suggestions.push(ColumnMapping {
                        source_column: header.clone(),
                        target_field: *field,
                    });
                    used_targets.insert(*field);
                    continue 'outer;
                }
            }
        }
        suggestions
    }

    /// Apply the status remapping to a HashMap of (source_value → count),
    /// returning a summary for display in the status-vocabulary step.
    pub fn remap_summary(&self, value_counts: &HashMap<String, usize>) -> Vec<RemapSummaryRow> {
        value_counts
            .iter()
            .map(|(source, count)| RemapSummaryRow {
                source_value: source.clone(),
                target_code: self.remap_status(source).to_string(),
                row_count: *count,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapSummaryRow {
    pub source_value: String,
    pub target_code: String,
    pub row_count: usize,
}
