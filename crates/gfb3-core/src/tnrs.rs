//! Taxonomic Name Resolution Service (TNRS) helpers + near-duplicate detection.
//!
//! HTTP calls live in the Tauri shell (`ureq`); this module formats requests,
//! parses responses, and flags ambiguous / near-duplicate species labels.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const TNRS_URL: &str = "https://tnrsapi.xyz/tnrs_api.php";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TnrsRequestBody {
    pub opts: TnrsOpts,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TnrsOpts {
    pub mode: String,
    pub matches: String,
    pub sources: String,
    pub class: String,
}

impl Default for TnrsOpts {
    fn default() -> Self {
        Self {
            mode: "resolve".into(),
            matches: "all".into(),
            sources: "wcvp,wfo".into(),
            class: "wfo".into(),
        }
    }
}

/// One unique species string from the dataset, ready for TNRS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesEntry {
    pub id: usize,
    pub original: String,
    pub normalized: String,
    pub row_count: usize,
    /// Other originals that look like near-duplicates of this string.
    pub near_duplicates: Vec<String>,
}

/// A single TNRS match candidate for manual resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TnrsMatch {
    pub name_submitted: String,
    pub name_matched: Option<String>,
    pub accepted_name: Option<String>,
    pub overall_score: Option<f64>,
    pub taxonomic_status: Option<String>,
    pub warnings: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TnrsResultRow {
    pub original: String,
    pub matches: Vec<TnrsMatch>,
    pub best_accepted: Option<String>,
    pub ambiguous: bool,
    pub near_duplicates: Vec<String>,
}

/// Normalize a species label for fuzzy comparison (lowercase, collapse spaces/punct).
pub fn normalize_species(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

/// True if two normalized labels are near-duplicates (same genus stem, small edit distance).
pub fn is_near_duplicate(a: &str, b: &str) -> bool {
    if a == b || a.is_empty() || b.is_empty() {
        return false;
    }
    let dist = levenshtein(a, b);
    let max_len = a.len().max(b.len());
    if max_len <= 4 {
        return dist == 1;
    }
    // Allow ~15% edit distance, at least 1 and at most 3
    let threshold = ((max_len as f64) * 0.15).ceil() as usize;
    dist >= 1 && dist <= threshold.max(1).min(3)
}

/// Build species entries with near-duplicate highlights from raw labels + counts.
pub fn build_species_entries(counts: &HashMap<String, usize>) -> Vec<SpeciesEntry> {
    let mut entries: Vec<SpeciesEntry> = counts
        .iter()
        .filter(|(k, _)| !k.trim().is_empty())
        .map(|(original, &row_count)| SpeciesEntry {
            id: 0,
            original: original.clone(),
            normalized: normalize_species(original),
            row_count,
            near_duplicates: vec![],
        })
        .collect();
    entries.sort_by(|a, b| b.row_count.cmp(&a.row_count).then(a.original.cmp(&b.original)));
    for (i, e) in entries.iter_mut().enumerate() {
        e.id = i + 1;
    }

    let norms: Vec<(String, String)> = entries
        .iter()
        .map(|e| (e.original.clone(), e.normalized.clone()))
        .collect();
    for e in &mut entries {
        let mut near = HashSet::new();
        for (other_orig, other_norm) in &norms {
            if other_orig == &e.original {
                continue;
            }
            if is_near_duplicate(&e.normalized, other_norm) {
                near.insert(other_orig.clone());
            }
        }
        e.near_duplicates = near.into_iter().collect();
        e.near_duplicates.sort();
    }
    entries
}

/// Build the JSON body for a TNRS resolve call (`matches=all` for ambiguity UI).
pub fn build_tnrs_request(entries: &[SpeciesEntry]) -> TnrsRequestBody {
    let data = entries
        .iter()
        .map(|e| format!("{}|{}", e.id, e.original))
        .collect();
    TnrsRequestBody {
        opts: TnrsOpts::default(),
        data,
    }
}

pub fn tnrs_url() -> &'static str {
    TNRS_URL
}

/// Parse a TNRS JSON response (array of objects) into result rows keyed by submitted name.
pub fn parse_tnrs_response(
    raw: &serde_json::Value,
    entries: &[SpeciesEntry],
) -> Result<Vec<TnrsResultRow>, String> {
    let arr = raw
        .as_array()
        .ok_or_else(|| "TNRS response is not a JSON array".to_string())?;

    let mut by_submitted: HashMap<String, Vec<TnrsMatch>> = HashMap::new();
    for item in arr {
        let name_submitted = item
            .get("Name_submitted")
            .or_else(|| item.get("name_submitted"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name_submitted.is_empty() {
            continue;
        }
        let m = TnrsMatch {
            name_submitted: name_submitted.clone(),
            name_matched: item
                .get("Name_matched")
                .or_else(|| item.get("name_matched"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            accepted_name: item
                .get("Accepted_name")
                .or_else(|| item.get("accepted_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            overall_score: item
                .get("Overall_score")
                .or_else(|| item.get("overall_score"))
                .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))),
            taxonomic_status: item
                .get("Taxonomic_status")
                .or_else(|| item.get("taxonomic_status"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            warnings: item
                .get("Warnings")
                .or_else(|| item.get("warnings"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        by_submitted.entry(name_submitted).or_default().push(m);
    }

    let mut rows = Vec::new();
    for e in entries {
        let matches = by_submitted.get(&e.original).cloned().unwrap_or_default();
        let mut accepted: Vec<String> = matches
            .iter()
            .filter_map(|m| m.accepted_name.clone())
            .filter(|s| !s.is_empty())
            .collect();
        accepted.sort();
        accepted.dedup();
        let best_accepted = matches
            .iter()
            .max_by(|a, b| {
                a.overall_score
                    .partial_cmp(&b.overall_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .and_then(|m| m.accepted_name.clone());
        let ambiguous = accepted.len() > 1
            || !e.near_duplicates.is_empty()
            || matches.iter().any(|m| {
                m.warnings
                    .as_deref()
                    .map(|w| !w.is_empty())
                    .unwrap_or(false)
            });
        rows.push(TnrsResultRow {
            original: e.original.clone(),
            matches,
            best_accepted,
            ambiguous,
            near_duplicates: e.near_duplicates.clone(),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_dup_detects_typo() {
        assert!(is_near_duplicate(
            &normalize_species("Ficus americana"),
            &normalize_species("Ficus americanus")
        ));
        assert!(!is_near_duplicate(
            &normalize_species("Ficus americana"),
            &normalize_species("Quercus robur")
        ));
    }

    #[test]
    fn build_entries_flags_near_dups() {
        let mut counts = HashMap::new();
        counts.insert("Ficus americana".into(), 10);
        counts.insert("Ficus americanus".into(), 2);
        let entries = build_species_entries(&counts);
        let a = entries.iter().find(|e| e.original == "Ficus americana").unwrap();
        assert!(a.near_duplicates.iter().any(|s| s == "Ficus americanus"));
    }
}
