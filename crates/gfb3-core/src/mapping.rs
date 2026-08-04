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
    /// Principal investigator email (optional).
    #[serde(default)]
    pub pi_email: Option<String>,
    /// Contributor / data contact display name.
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
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
    /// Exact / strong aliases win over weak substring matches so a real
    /// `TreeID` column is not stolen by something like `subplot_id` via `"id"`.
    pub fn suggest_from_headers(headers: &[String]) -> Vec<ColumnMapping> {
        // exact: normalized header equality
        // strong: normalized header contains alias (prefer longer tokens)
        // weak: last-resort contains (short / ambiguous tokens)
        let rules: &[(Gfb3Field, &[&str], &[&str], &[&str])] = &[
            (
                Gfb3Field::PlotId,
                &["plotid", "plot_id"],
                &["plot"],
                &["site"],
            ),
            (
                Gfb3Field::TreeId,
                &["treeid", "tree_id"],
                &["tree", "individual"],
                &[], // never use bare "id" — matches subplot_id, register_unit_id, etc.
            ),
            (
                Gfb3Field::Yr,
                &["yr", "year"],
                &["census_year", "survey_year", "measurement_date", "survey_date", "fecha_censo"],
                &["date", "fecha", "datetime"],
            ),
            (
                Gfb3Field::PrevYr,
                &["prevyr", "prev_yr", "previous_year", "prev_year"],
                &[],
                &[],
            ),
            (
                Gfb3Field::Status,
                &["status"],
                &["condition", "fate"],
                &["state"],
            ),
            (
                Gfb3Field::Dbh,
                &["dbh", "d130", "d_bh"],
                &["diameter", "diam"],
                &[],
            ),
            (
                Gfb3Field::Species,
                &["species", "taxon", "spp"],
                &["scientific"],
                &["sp"],
            ),
        ];

        let norms: Vec<(String, String)> = headers
            .iter()
            .map(|h| {
                (
                    h.clone(),
                    h.to_lowercase().replace([' ', '-'], "_"),
                )
            })
            .collect();

        let mut suggestions = Vec::new();
        let mut used_targets = std::collections::HashSet::new();
        let mut used_sources = std::collections::HashSet::new();

        // Pass 1 — exact canonical / strong names
        for (orig, norm) in &norms {
            for (field, exact, _, _) in rules {
                if used_targets.contains(field) || used_sources.contains(orig) {
                    continue;
                }
                if exact.iter().any(|a| norm == *a) {
                    suggestions.push(ColumnMapping {
                        source_column: orig.clone(),
                        target_field: *field,
                    });
                    used_targets.insert(*field);
                    used_sources.insert(orig.clone());
                }
            }
        }

        // Pass 2 — strong substring aliases (longest alias first per field)
        for (orig, norm) in &norms {
            if used_sources.contains(orig) {
                continue;
            }
            for (field, _, strong, _) in rules {
                if used_targets.contains(field) {
                    continue;
                }
                let mut aliases: Vec<&&str> = strong.iter().collect();
                aliases.sort_by_key(|a| std::cmp::Reverse(a.len()));
                if aliases.iter().any(|a| norm.contains(**a)) {
                    suggestions.push(ColumnMapping {
                        source_column: orig.clone(),
                        target_field: *field,
                    });
                    used_targets.insert(*field);
                    used_sources.insert(orig.clone());
                }
            }
        }

        // Pass 3 — weak aliases
        for (orig, norm) in &norms {
            if used_sources.contains(orig) {
                continue;
            }
            for (field, _, _, weak) in rules {
                if used_targets.contains(field) || weak.is_empty() {
                    continue;
                }
                if weak.iter().any(|a| norm.contains(*a)) {
                    suggestions.push(ColumnMapping {
                        source_column: orig.clone(),
                        target_field: *field,
                    });
                    used_targets.insert(*field);
                    used_sources.insert(orig.clone());
                }
            }
        }

        suggestions
    }

    /// Suggest plot metadata columns (Latitude / Longitude / PA) by exact header match.
    /// Short names like "lat" / "pa" use equality, not substring, to avoid false hits.
    pub fn suggest_plot_meta_from_headers(headers: &[String]) -> Vec<(String, String)> {
        let rules: &[(&str, &[&str])] = &[
            (
                "Latitude",
                &[
                    "lat",
                    "latitude",
                    "latitud",
                    "y_lat",
                    "coord_y",
                    "coords_y",
                ],
            ),
            (
                "Longitude",
                &[
                    "lon",
                    "long",
                    "longitude",
                    "longitud",
                    "lng",
                    "x_lon",
                    "x_long",
                    "coord_x",
                    "coords_x",
                ],
            ),
            ("PA", &["pa", "plot_area", "plotarea", "area_ha", "plot_ha"]),
        ];
        let mut out = Vec::new();
        let mut used_targets = std::collections::HashSet::new();
        let mut used_sources = std::collections::HashSet::new();

        for header in headers {
            let norm = header.to_lowercase().replace([' ', '-'], "_");
            for (target, aliases) in rules {
                if used_targets.contains(target) || used_sources.contains(header.as_str()) {
                    continue;
                }
                if aliases.iter().any(|a| norm == *a) {
                    out.push((header.clone(), (*target).to_string()));
                    used_targets.insert(*target);
                    used_sources.insert(header.as_str());
                    break;
                }
            }
        }
        out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treeid_exact_wins_over_subplot_id() {
        let headers = vec![
            "subplot_subplot_id".into(),
            "TreeID".into(),
            "PlotID".into(),
            "DBH".into(),
        ];
        let s = ContributorMapping::suggest_from_headers(&headers);
        let tree = s.iter().find(|m| m.target_field == Gfb3Field::TreeId).unwrap();
        assert_eq!(tree.source_column, "TreeID");
        let plot = s.iter().find(|m| m.target_field == Gfb3Field::PlotId).unwrap();
        assert_eq!(plot.source_column, "PlotID");
    }

    #[test]
    fn treeid_case_insensitive_exact() {
        let headers = vec!["tree_id".into(), "register_unit_id".into()];
        let s = ContributorMapping::suggest_from_headers(&headers);
        let tree = s.iter().find(|m| m.target_field == Gfb3Field::TreeId).unwrap();
        assert_eq!(tree.source_column, "tree_id");
    }
}
