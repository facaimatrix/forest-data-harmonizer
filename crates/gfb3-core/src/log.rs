use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::mapping::DatasetMetadata;
use crate::validation::{ValidationFinding, ValidationRule};

/// Fixed curation-log template, partially pre-filled from the guided flow.
///
/// Sections that require PI-level judgment are left blank for the curator.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurationLog {
    pub dataset: Option<String>,
    pub country: Option<String>,
    pub site: Option<String>,
    pub pi: Option<String>,
    pub curator: String,
    pub date_received: Option<String>,
    pub date_processed: Option<String>,
    pub source_format: Vec<String>,
    pub pivot_restructuring: Vec<String>,
    pub duplicate_resolution: Vec<String>,
    pub missing_interpolated: Vec<String>,
    pub species_issues: Vec<String>,
    pub exclusions: Vec<String>,
    pub notes: Vec<String>,
}

impl CurationLog {
    pub fn new(curator: &str) -> Self {
        CurationLog {
            curator: curator.to_string(),
            date_processed: Some(Utc::now().format("%Y-%m-%d").to_string()),
            ..Default::default()
        }
    }

    /// Pre-fill fields that are derivable from the contributor mapping + metadata.
    pub fn prefill_from_metadata(&mut self, meta: &DatasetMetadata, dsn: &str) {
        self.dataset = Some(dsn.to_string());
        self.country = meta.country.clone();
        self.site = meta.site.clone();
        self.pi = meta.pi.clone();
    }

    /// Append curation entries from Escalate-severity validation findings.
    pub fn append_escalated_findings(&mut self, findings: &[ValidationFinding]) {
        for f in findings {
            if matches!(f.rule, ValidationRule::RecruitAtMinYear | ValidationRule::NoPersistentTreeId) {
                self.notes.push(format!(
                    "[AUTO-FLAGGED] {} ({} rows) — {}",
                    format!("{:?}", f.rule),
                    f.row_count,
                    f.message
                ));
            }
        }
    }

    /// Render to the fixed plain-text template format.
    pub fn render(&self) -> String {
        let mut out = String::new();

        fn opt(v: &Option<String>) -> &str {
            v.as_deref().unwrap_or("")
        }
        fn lines(v: &[String]) -> String {
            if v.is_empty() {
                String::new()
            } else {
                v.iter().map(|s| format!("  {}", s)).collect::<Vec<_>>().join("\n")
            }
        }

        out.push_str(&format!("DATASET: {}\n", opt(&self.dataset)));
        out.push_str(&format!("COUNTRY: {}\n", opt(&self.country)));
        out.push_str(&format!("SITE: {}\n", opt(&self.site)));
        out.push_str(&format!("PI: {}\n", opt(&self.pi)));
        out.push_str(&format!("CURATOR: {}\n", self.curator));
        out.push_str(&format!("DATE RECEIVED: {}\n", opt(&self.date_received)));
        out.push_str(&format!("DATE PROCESSED: {}\n", opt(&self.date_processed)));
        out.push_str("--- SOURCE FORMAT ---\n");
        out.push_str(&lines(&self.source_format));
        out.push('\n');
        out.push_str("--- PIVOT / RESTRUCTURING ---\n");
        out.push_str(&lines(&self.pivot_restructuring));
        out.push('\n');
        out.push_str("--- DUPLICATE RESOLUTION ---\n");
        out.push_str(&lines(&self.duplicate_resolution));
        out.push('\n');
        out.push_str("--- MISSING / INTERPOLATED DATA ---\n");
        out.push_str(&lines(&self.missing_interpolated));
        out.push('\n');
        out.push_str("--- SPECIES ISSUES ---\n");
        out.push_str(&lines(&self.species_issues));
        out.push('\n');
        out.push_str("--- EXCLUSIONS ---\n");
        out.push_str(&lines(&self.exclusions));
        out.push('\n');
        out.push_str("--- NOTES ---\n");
        out.push_str(&lines(&self.notes));
        out.push('\n');

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_curator_header() {
        let log = CurationLog::new("Francisco Rivas");
        let rendered = log.render();
        assert!(rendered.contains("CURATOR: Francisco Rivas"));
        assert!(rendered.contains("--- SOURCE FORMAT ---"));
    }
}
