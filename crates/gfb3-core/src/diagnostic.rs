//! Facai-style GFB3 / GFB2 diagnostic report.
//!
//! Multi-census (GFB3) runs the full suite including Prev* consistency and growth.
//! Single-census (GFB2) skips Prev*/growth checks that require paired inventories.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mapping::CensusType;
use crate::schema::{STATUS_ALIVE, STATUS_DEAD, STATUS_RECRUIT};

const UNIDENTIFIED_DEFAULT: &str = "Unidentified sp.";

#[derive(Debug, Error)]
pub enum DiagnosticError {
    #[error("Polars error during diagnostic: {0}")]
    Polars(#[from] PolarsError),
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRow {
    pub status: String,
    pub label: String,
    pub n: u64,
    pub pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagRow {
    pub flag: String,
    pub count: f64,
    pub severity: String, // critical | warning | info
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaRow {
    pub plot_id: String,
    pub yr: f64,
    pub ba: f64,
    pub ba_flag: String, // ok | warning | critical
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TphRow {
    pub plot_id: String,
    pub yr: f64,
    pub n_trees: u64,
    pub pa: f64,
    pub tph: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotCountRow {
    pub plot_id: String,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbhSummary {
    pub n: u64,
    pub mean: Option<f64>,
    pub sd: Option<f64>,
    pub min: Option<f64>,
    pub q25: Option<f64>,
    pub median: Option<f64>,
    pub q75: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSummary {
    pub n: u64,
    pub mean_delta: Option<f64>,
    pub mean_annual: Option<f64>,
    pub pct_negative: f64,
    pub n_zero: u64,
    pub n_fast: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCharts {
    pub dbh_hist_svg: String,
    pub growth_hist_svg: Option<String>,
    pub ba_bar_svg: String,
    pub tph_bar_svg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub dataset_name: String,
    pub census_type: String,
    pub n_rows: u64,
    pub n_trees: u64,
    pub n_plots: u64,
    pub yr_min: Option<f64>,
    pub yr_max: Option<f64>,
    pub status: Vec<StatusRow>,
    pub flags: Vec<FlagRow>,
    pub ba: Vec<BaRow>,
    pub tph: Vec<TphRow>,
    pub na_pa: Vec<PlotCountRow>,
    pub na_species: Vec<PlotCountRow>,
    pub unidentified: Vec<PlotCountRow>,
    pub dbh: DbhSummary,
    pub growth: Option<GrowthSummary>,
    pub charts: DiagnosticCharts,
    pub verdict: String,
    pub verdict_level: String, // pass | warn | fail
    pub curation_log: Option<String>,
    pub html: String,
}

fn status_label(s: &str) -> &'static str {
    match s {
        "0" => "alive",
        "1" => "dead between inventories",
        "2" => "new recruit",
        "9" => "missing",
        _ => "unknown",
    }
}

fn has_col(df: &DataFrame, name: &str) -> bool {
    df.get_column_names().iter().any(|c| c.as_str() == name)
}

fn col_str(df: &DataFrame, name: &str, i: usize) -> String {
    df.column(name)
        .ok()
        .and_then(|c| c.get(i).ok())
        .map(|v| match v {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn col_f64(df: &DataFrame, name: &str, i: usize) -> Option<f64> {
    df.column(name).ok().and_then(|c| c.get(i).ok()).and_then(|v| match v {
        AnyValue::Float64(x) => Some(x),
        AnyValue::Float32(x) => Some(x as f64),
        AnyValue::Int64(x) => Some(x as f64),
        AnyValue::Int32(x) => Some(x as f64),
        AnyValue::UInt32(x) => Some(x as f64),
        AnyValue::UInt64(x) => Some(x as f64),
        AnyValue::String(s) => s.trim().parse().ok(),
        AnyValue::StringOwned(s) => s.trim().parse().ok(),
        _ => None,
    })
}

fn col_u64(df: &DataFrame, name: &str, i: usize) -> u64 {
    col_f64(df, name, i).unwrap_or(0.0) as u64
}

fn species_malformed(species: &str, unid: &str) -> bool {
    if species.is_empty() || species.eq_ignore_ascii_case("null") {
        return true;
    }
    if !unid.is_empty() && species == unid {
        return false;
    }
    // Genus Species pattern: Capital + lowercase + space + lowercase
    let bytes = species.as_bytes();
    if bytes.len() < 3 {
        return true;
    }
    if !(bytes[0].is_ascii_uppercase()) {
        return true;
    }
    let Some(sp) = species.find(' ') else {
        return true;
    };
    let genus = &species[..sp];
    let epithet = species[sp + 1..].trim();
    if genus.len() < 2 || !genus[1..].chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        return true;
    }
    if epithet.is_empty() || !epithet.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return true;
    }
    false
}

fn quantile_sorted(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len();
    let idx = ((n as f64 - 1.0) * q).round() as usize;
    Some(sorted[idx.min(n - 1)])
}

fn dbh_summary_from_vals(vals: &mut [f64]) -> DbhSummary {
    let n = vals.len() as u64;
    if vals.is_empty() {
        return DbhSummary {
            n: 0,
            mean: None,
            sd: None,
            min: None,
            q25: None,
            median: None,
            q75: None,
            max: None,
        };
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    DbhSummary {
        n,
        mean: Some(mean),
        sd: Some(var.sqrt()),
        min: vals.first().copied(),
        q25: quantile_sorted(vals, 0.25),
        median: quantile_sorted(vals, 0.5),
        q75: quantile_sorted(vals, 0.75),
        max: vals.last().copied(),
    }
}

fn hist_svg(vals: &[f64], title: &str, n_bins: usize, width: u32, height: u32) -> String {
    if vals.is_empty() {
        return format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><text x="12" y="24" fill="#5f7068">{title} — no data</text></svg>"##,
            w = width,
            h = height,
            title = title
        );
    }
    let min_v = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_v - min_v).max(1e-9);
    let mut bins = vec![0u64; n_bins];
    for &v in vals {
        let mut i = ((v - min_v) / span * n_bins as f64).floor() as usize;
        if i >= n_bins {
            i = n_bins - 1;
        }
        bins[i] += 1;
    }
    let max_count = bins.iter().copied().max().unwrap_or(1).max(1);
    let left = 40.0;
    let top = 28.0;
    let bottom = 28.0;
    let right = 12.0;
    let plot_w = width as f64 - left - right;
    let plot_h = height as f64 - top - bottom;
    let bar_w = plot_w / n_bins as f64;
    let mut rects = String::new();
    for (i, &c) in bins.iter().enumerate() {
        let h = plot_h * (c as f64 / max_count as f64);
        let x = left + i as f64 * bar_w;
        let y = top + plot_h - h;
        rects.push_str(&format!(
            r##"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="#40916c"/>"##,
            w = (bar_w * 0.9).max(1.0)
        ));
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <text x="12" y="18" font-family="system-ui,sans-serif" font-size="12" fill="#1b4332" font-weight="600">{title}</text>
  <line x1="{left}" y1="{y0}" x2="{x1}" y2="{y0}" stroke="#d0d5d2"/>
  {rects}
  <text x="{left}" y="{yl}" font-size="10" fill="#5f7068">{min:.1}</text>
  <text x="{xr}" y="{yl}" font-size="10" fill="#5f7068" text-anchor="end">{max:.1}</text>
</svg>"##,
        y0 = top + plot_h,
        x1 = left + plot_w,
        yl = height as f64 - 8.0,
        xr = left + plot_w,
        min = min_v,
        max = max_v,
    )
}

fn bar_svg(
    labels: &[String],
    values: &[f64],
    colors: &[String],
    title: &str,
    ylab: &str,
    width: u32,
    height: u32,
    hlines: &[(f64, &str)],
) -> String {
    if labels.is_empty() {
        return format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><text x="12" y="24" fill="#5f7068">{title} — no data</text></svg>"##,
            w = width,
            h = height,
            title = title
        );
    }
    let max_v = values
        .iter()
        .cloned()
        .chain(hlines.iter().map(|(v, _)| *v))
        .fold(0.0_f64, f64::max)
        .max(1e-9)
        * 1.15;
    let left = 48.0;
    let top = 28.0;
    let bottom = 56.0;
    let right = 12.0;
    let plot_w = width as f64 - left - right;
    let plot_h = height as f64 - top - bottom;
    let bar_w = plot_w / labels.len() as f64;
    let mut parts = String::new();
    for &(y, color) in hlines {
        let yy = top + plot_h * (1.0 - y / max_v);
        parts.push_str(&format!(
            r#"<line x1="{left}" y1="{yy:.1}" x2="{x1}" y2="{yy:.1}" stroke="{color}" stroke-dasharray="4 3" stroke-width="1"/>"#,
            x1 = left + plot_w
        ));
    }
    for (i, ((lab, val), col)) in labels.iter().zip(values.iter()).zip(colors.iter()).enumerate() {
        let h = plot_h * (*val / max_v);
        let x = left + i as f64 * bar_w + bar_w * 0.1;
        let y = top + plot_h - h;
        parts.push_str(&format!(
            r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{col}"/>"#,
            w = (bar_w * 0.8).max(1.0)
        ));
        let lx = left + i as f64 * bar_w + bar_w * 0.5;
        parts.push_str(&format!(
            r##"<text x="{lx:.1}" y="{ty}" font-size="8" fill="#5f7068" text-anchor="middle" transform="rotate(-65 {lx:.1} {ty})">{lab}</text>"##,
            ty = height as f64 - 8.0
        ));
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <text x="12" y="18" font-family="system-ui,sans-serif" font-size="12" fill="#1b4332" font-weight="600">{title}</text>
  <text x="8" y="{ym}" font-size="9" fill="#5f7068" transform="rotate(-90 8 {ym})">{ylab}</text>
  <line x1="{left}" y1="{y0}" x2="{x1}" y2="{y0}" stroke="#d0d5d2"/>
  {parts}
</svg>"##,
        ym = top + plot_h * 0.5,
        y0 = top + plot_h,
        x1 = left + plot_w,
    )
}

/// Build a facai-style diagnostic report from a mapped GFB2/GFB3 frame.
pub fn build_diagnostic_report(
    df: &DataFrame,
    census_type: CensusType,
    dataset_name: &str,
    curation_log: Option<&str>,
    unidentified_label: Option<&str>,
) -> Result<DiagnosticReport, DiagnosticError> {
    let unid = unidentified_label.unwrap_or(UNIDENTIFIED_DEFAULT);
    let is_multi = census_type == CensusType::Multi;
    let n_rows = df.height() as u64;

    if !has_col(df, "PlotID") || !has_col(df, "TreeID") || !has_col(df, "YR") {
        return Err(DiagnosticError::Msg(
            "diagnostic requires PlotID, TreeID, and YR columns".into(),
        ));
    }

    let n_trees = df
        .clone()
        .lazy()
        .select([col("TreeID").n_unique().alias("n")])
        .collect()?
        .column("n")?
        .get(0)
        .ok()
        .and_then(|v| match v {
            AnyValue::UInt32(x) => Some(x as u64),
            AnyValue::UInt64(x) => Some(x),
            AnyValue::Int64(x) => Some(x as u64),
            _ => None,
        })
        .unwrap_or(0);

    let n_plots = df
        .clone()
        .lazy()
        .select([col("PlotID").n_unique().alias("n")])
        .collect()?
        .column("n")?
        .get(0)
        .ok()
        .and_then(|v| match v {
            AnyValue::UInt32(x) => Some(x as u64),
            AnyValue::UInt64(x) => Some(x),
            AnyValue::Int64(x) => Some(x as u64),
            _ => None,
        })
        .unwrap_or(0);

    let mut yrs: Vec<f64> = Vec::new();
    for i in 0..df.height() {
        if let Some(y) = col_f64(df, "YR", i) {
            yrs.push(y);
        }
        if is_multi && has_col(df, "PrevYR") {
            if let Some(y) = col_f64(df, "PrevYR", i) {
                yrs.push(y);
            }
        }
    }
    let yr_min = yrs.iter().cloned().reduce(f64::min);
    let yr_max = yrs.iter().cloned().reduce(f64::max);

    // Status distribution
    let mut status: Vec<StatusRow> = Vec::new();
    if has_col(df, "Status") {
        let st = df
            .clone()
            .lazy()
            .group_by([col("Status").cast(DataType::String)])
            .agg([len().alias("n")])
            .sort(["Status"], SortMultipleOptions::default())
            .collect()?;
        let total = (0..st.height()).map(|i| col_u64(&st, "n", i)).sum::<u64>().max(1);
        for i in 0..st.height() {
            let s = col_str(&st, "Status", i);
            let n = col_u64(&st, "n", i);
            status.push(StatusRow {
                label: status_label(&s).to_string(),
                status: s,
                n,
                pct: (100.0 * n as f64 / total as f64 * 10.0).round() / 10.0,
            });
        }
    }

    // Collect row-level values for flags
    let mut dbh_vals: Vec<f64> = Vec::new();
    let mut n_missing_yr = 0u64;
    let mut n_missing_dbh = 0u64;
    let mut n_missing_prevdbh = 0u64;
    let mut n_small = 0u64;
    let mut n_missing_species_trees = std::collections::HashSet::<String>::new();
    let mut growth_deltas: Vec<f64> = Vec::new();
    let mut growth_annual: Vec<f64> = Vec::new();
    let mut n_zero_growth = 0u64;
    let mut n_fast = 0u64;
    let mut n_neg = 0u64;
    let mut n_growth_eligible = 0u64;

    for i in 0..df.height() {
        let status_i = if has_col(df, "Status") {
            col_str(df, "Status", i)
        } else {
            STATUS_ALIVE.to_string()
        };
        let tree = col_str(df, "TreeID", i);
        if col_f64(df, "YR", i).is_none() {
            n_missing_yr += 1;
        }
        let dbh = if has_col(df, "DBH") {
            col_f64(df, "DBH", i)
        } else {
            None
        };
        if matches!(status_i.as_str(), s if s == STATUS_ALIVE || s == STATUS_RECRUIT) && dbh.is_none()
        {
            n_missing_dbh += 1;
        }
        if let Some(d) = dbh {
            dbh_vals.push(d);
            if d < 10.0 {
                n_small += 1;
            }
        }
        if is_multi && status_i == STATUS_ALIVE && has_col(df, "PrevDBH") && col_f64(df, "PrevDBH", i).is_none()
        {
            // recruits excluded later conceptually; count missing PrevDBH on alive
            n_missing_prevdbh += 1;
        }
        if has_col(df, "Species") {
            let sp = col_str(df, "Species", i);
            if species_malformed(&sp, unid) {
                n_missing_species_trees.insert(tree.clone());
            }
        } else {
            n_missing_species_trees.insert(tree.clone());
        }

        if is_multi && has_col(df, "PrevDBH") {
            if let (Some(d), Some(pd)) = (dbh, col_f64(df, "PrevDBH", i)) {
                let delta = d - pd;
                if status_i == STATUS_ALIVE {
                    n_growth_eligible += 1;
                    growth_deltas.push(delta);
                    if delta < 0.0 {
                        n_neg += 1;
                    }
                }
                if (delta).abs() < 1e-12 {
                    n_zero_growth += 1;
                }
                if let (Some(yr), Some(py)) = (col_f64(df, "YR", i), col_f64(df, "PrevYR", i)) {
                    let dy = yr - py;
                    if dy != 0.0 {
                        let ann = delta / dy;
                        growth_annual.push(ann);
                        if ann > 5.0 {
                            n_fast += 1;
                        }
                    }
                }
            }
        }
    }

    // Recruits shouldn't count as missing PrevDBH — adjust: recompute missing PrevDBH excluding recruits
    if is_multi && has_col(df, "PrevDBH") && has_col(df, "Status") {
        n_missing_prevdbh = 0;
        for i in 0..df.height() {
            let st = col_str(df, "Status", i);
            if st == STATUS_ALIVE && col_f64(df, "PrevDBH", i).is_none() {
                n_missing_prevdbh += 1;
            }
        }
    }

    let pct_neg = if n_growth_eligible > 0 {
        (100.0 * n_neg as f64 / n_growth_eligible as f64 * 100.0).round() / 100.0
    } else {
        0.0
    };

    // Duplicates
    let dup_df = df
        .clone()
        .lazy()
        .group_by([col("PlotID"), col("TreeID"), col("YR")])
        .agg([len().alias("n")])
        .filter(col("n").gt(lit(1)))
        .collect()?;
    let n_dups = dup_df.height() as u64;

    // Zombies (multi)
    let mut n_zombie = 0u64;
    if is_multi && has_col(df, "Status") {
        let sorted = df
            .clone()
            .lazy()
            .sort(
                ["PlotID", "TreeID", "YR"],
                SortMultipleOptions::default().with_order_descending_multi([false, false, false]),
            )
            .collect()?;
        let mut last_key = String::new();
        let mut ever_dead = false;
        for i in 0..sorted.height() {
            let key = format!("{}||{}", col_str(&sorted, "PlotID", i), col_str(&sorted, "TreeID", i));
            if key != last_key {
                last_key = key;
                ever_dead = false;
            }
            let st = col_str(&sorted, "Status", i);
            if st == STATUS_DEAD {
                ever_dead = true;
            } else if ever_dead && (st == STATUS_ALIVE || st == STATUS_RECRUIT) {
                n_zombie += 1;
            }
        }
    }

    // Prev consistency (multi)
    let mut n_prevdbh_mismatch = 0u64;
    let mut n_prevyr_mismatch = 0u64;
    let mut n_prevdbh_orphan = 0u64;
    let mut n_prevyr_orphan = 0u64;
    if is_multi && has_col(df, "PrevYR") {
        let sorted = df
            .clone()
            .lazy()
            .sort(
                ["PlotID", "TreeID", "YR"],
                SortMultipleOptions::default().with_order_descending_multi([false, false, false]),
            )
            .collect()?;
        let mut last_key = String::new();
        let mut lag_dbh: Option<f64> = None;
        let mut lag_yr: Option<f64> = None;
        for i in 0..sorted.height() {
            let key = format!("{}||{}", col_str(&sorted, "PlotID", i), col_str(&sorted, "TreeID", i));
            if key != last_key {
                last_key = key;
                lag_dbh = None;
                lag_yr = None;
            }
            let st = if has_col(&sorted, "Status") {
                col_str(&sorted, "Status", i)
            } else {
                STATUS_ALIVE.to_string()
            };
            let dbh = if has_col(&sorted, "DBH") {
                col_f64(&sorted, "DBH", i)
            } else {
                None
            };
            let yr = col_f64(&sorted, "YR", i);
            let prev_dbh = if has_col(&sorted, "PrevDBH") {
                col_f64(&sorted, "PrevDBH", i)
            } else {
                None
            };
            let prev_yr = col_f64(&sorted, "PrevYR", i);

            if st == STATUS_ALIVE {
                if let (Some(pd), Some(ld)) = (prev_dbh, lag_dbh) {
                    if (pd - ld).abs() > 1e-6 {
                        n_prevdbh_mismatch += 1;
                    }
                }
                if let (Some(py), Some(ly)) = (prev_yr, lag_yr) {
                    if (py - ly).abs() > 1e-6 {
                        n_prevyr_mismatch += 1;
                    }
                }
                if prev_dbh.is_none() && lag_dbh.is_some() && dbh.is_some() {
                    n_prevdbh_orphan += 1;
                }
                if prev_yr.is_none() && lag_yr.is_some() && dbh.is_some() {
                    n_prevyr_orphan += 1;
                }
            }
            lag_dbh = dbh;
            lag_yr = yr;
        }
    }

    // BA / TPH (manual aggregation — robust across dtypes; single + multi)
    let mut ba: Vec<BaRow> = Vec::new();
    let mut tph: Vec<TphRow> = Vec::new();
    let mut ba_skip_reason: Option<&'static str> = None;
    if !has_col(df, "DBH") {
        ba_skip_reason = Some("DBH column missing");
    } else if !has_col(df, "PA") {
        ba_skip_reason = Some("PA (plot area, ha) not mapped — assign a column or constant in Field assignment");
    } else {
        let mut map: std::collections::BTreeMap<(String, i64), (f64, f64, u64, f64)> =
            std::collections::BTreeMap::new();
        let mut n_alive_with_dbh = 0u64;
        for i in 0..df.height() {
            // Missing Status → treat as alive (typical for single-census before/without Status).
            // Null/blank Status likewise — do not drop stems from BA/TPH.
            let st = if has_col(df, "Status") {
                let s = col_str(df, "Status", i);
                let t = s.trim();
                if t.is_empty() || t.eq_ignore_ascii_case("null") {
                    STATUS_ALIVE.to_string()
                } else {
                    t.to_string()
                }
            } else {
                STATUS_ALIVE.to_string()
            };
            if st != STATUS_ALIVE {
                continue;
            }
            let Some(dbh) = col_f64(df, "DBH", i) else { continue };
            let Some(pa) = col_f64(df, "PA", i) else { continue };
            if pa <= 0.0 {
                continue;
            }
            let Some(yr) = col_f64(df, "YR", i) else { continue };
            n_alive_with_dbh += 1;
            let plot = col_str(df, "PlotID", i);
            let key = (plot, yr.floor() as i64);
            let e = map.entry(key).or_insert((0.0, pa, 0, yr));
            e.0 += std::f64::consts::PI * (dbh / 200.0).powi(2);
            e.1 = pa;
            e.2 += 1;
            e.3 = yr;
        }
        if map.is_empty() {
            ba_skip_reason = Some(if n_alive_with_dbh == 0 {
                "no alive stems with valid DBH and PA > 0"
            } else {
                "could not aggregate BA/TPH"
            });
        }
        for ((plot, _), (ba_sum, pa, n, yr)) in map {
            let ba_ha = ba_sum / pa;
            let flag = if ba_ha >= 100.0 {
                "critical"
            } else if ba_ha > 50.0 {
                "warning"
            } else {
                "ok"
            };
            ba.push(BaRow {
                plot_id: plot.clone(),
                yr,
                ba: (ba_ha * 1000.0).round() / 1000.0,
                ba_flag: flag.into(),
            });
            tph.push(TphRow {
                plot_id: plot,
                yr,
                n_trees: n,
                pa,
                tph: n as f64 / pa,
            });
        }
    }

    let n_ba_warning = ba.iter().filter(|r| r.ba_flag == "warning").count() as u64;
    let n_ba_critical = ba.iter().filter(|r| r.ba_flag == "critical").count() as u64;

    // NA PA / species / unidentified by plot
    let mut na_pa_map: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut na_sp_map: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
        std::collections::BTreeMap::new();
    let mut unid_map: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
        std::collections::BTreeMap::new();
    for i in 0..df.height() {
        let plot = col_str(df, "PlotID", i);
        let tree = col_str(df, "TreeID", i);
        if has_col(df, "PA") && col_f64(df, "PA", i).is_none() {
            *na_pa_map.entry(plot.clone()).or_default() += 1;
        }
        if has_col(df, "Species") {
            let sp = col_str(df, "Species", i);
            if species_malformed(&sp, unid) {
                na_sp_map.entry(plot.clone()).or_default().insert(tree.clone());
            }
            if !unid.is_empty() && sp == unid {
                unid_map.entry(plot).or_default().insert(tree);
            }
        }
    }
    let mut na_pa: Vec<PlotCountRow> = na_pa_map
        .into_iter()
        .map(|(plot_id, n)| PlotCountRow { plot_id, n })
        .collect();
    na_pa.sort_by(|a, b| b.n.cmp(&a.n));
    let mut na_species: Vec<PlotCountRow> = na_sp_map
        .into_iter()
        .map(|(plot_id, s)| PlotCountRow {
            plot_id,
            n: s.len() as u64,
        })
        .collect();
    na_species.sort_by(|a, b| b.n.cmp(&a.n));
    let mut unidentified: Vec<PlotCountRow> = unid_map
        .into_iter()
        .map(|(plot_id, s)| PlotCountRow {
            plot_id,
            n: s.len() as u64,
        })
        .collect();
    unidentified.sort_by(|a, b| b.n.cmp(&a.n));

    let dbh = dbh_summary_from_vals(&mut dbh_vals);

    let growth = if is_multi {
        let mean_delta = if growth_deltas.is_empty() {
            None
        } else {
            Some(growth_deltas.iter().sum::<f64>() / growth_deltas.len() as f64)
        };
        let mean_annual = if growth_annual.is_empty() {
            None
        } else {
            Some(growth_annual.iter().sum::<f64>() / growth_annual.len() as f64)
        };
        Some(GrowthSummary {
            n: growth_deltas.len() as u64,
            mean_delta,
            mean_annual,
            pct_negative: pct_neg,
            n_zero: n_zero_growth,
            n_fast,
        })
    } else {
        None
    };

    let mut flags = vec![
        FlagRow {
            flag: "Missing YR".into(),
            count: n_missing_yr as f64,
            severity: "critical".into(),
        },
        FlagRow {
            flag: "Missing DBH (dead trees excluded)".into(),
            count: n_missing_dbh as f64,
            severity: "warning".into(),
        },
        FlagRow {
            flag: "Missing/malformed Species".into(),
            count: n_missing_species_trees.len() as f64,
            severity: "warning".into(),
        },
        FlagRow {
            flag: "DBH < 10 cm".into(),
            count: n_small as f64,
            severity: "warning".into(),
        },
        FlagRow {
            flag: "Duplicate TreeID × YR".into(),
            count: n_dups as f64,
            severity: "critical".into(),
        },
        FlagRow {
            flag: "BA 51–100 m²/ha (higher than typical)".into(),
            count: n_ba_warning as f64,
            severity: "warning".into(),
        },
        FlagRow {
            flag: "BA ≥ 100 m²/ha (implausible)".into(),
            count: n_ba_critical as f64,
            severity: "critical".into(),
        },
    ];
    if is_multi {
        flags.extend([
            FlagRow {
                flag: "Missing PrevDBH (recruits excluded)".into(),
                count: n_missing_prevdbh as f64,
                severity: "info".into(),
            },
            FlagRow {
                flag: "Negative DBH growth (%)".into(),
                count: pct_neg,
                severity: "warning".into(),
            },
            FlagRow {
                flag: "Zero DBH growth".into(),
                count: n_zero_growth as f64,
                severity: "info".into(),
            },
            FlagRow {
                flag: "Annual growth > 5 cm/yr".into(),
                count: n_fast as f64,
                severity: "warning".into(),
            },
            FlagRow {
                flag: "Zombie trees (alive after death)".into(),
                count: n_zombie as f64,
                severity: "critical".into(),
            },
            FlagRow {
                flag: "PrevDBH value mismatches lag(DBH)".into(),
                count: n_prevdbh_mismatch as f64,
                severity: "critical".into(),
            },
            FlagRow {
                flag: "PrevYR value mismatches lag(YR)".into(),
                count: n_prevyr_mismatch as f64,
                severity: "critical".into(),
            },
            FlagRow {
                flag: "PrevDBH missing but valid lag(DBH) exists (alive)".into(),
                count: n_prevdbh_orphan as f64,
                severity: "critical".into(),
            },
            FlagRow {
                flag: "PrevYR missing but valid lag(YR) exists (alive)".into(),
                count: n_prevyr_orphan as f64,
                severity: "critical".into(),
            },
        ]);
    }

    let hard = n_missing_yr
        + n_dups
        + n_ba_critical
        + n_zombie
        + n_prevdbh_mismatch
        + n_prevyr_mismatch
        + n_prevdbh_orphan
        + n_prevyr_orphan;
    let soft = n_missing_dbh + n_small + n_fast + n_ba_warning + if pct_neg > 0.0 { 1 } else { 0 };
    let (verdict, verdict_level) = if hard > 0 {
        (
            "FAIL: critical issues must be resolved before use.".to_string(),
            "fail".to_string(),
        )
    } else if soft > 0 {
        (
            "WARN: passed critical checks but has warnings worth reviewing.".to_string(),
            "warn".to_string(),
        )
    } else {
        (
            "PASS: dataset looks clean.".to_string(),
            "pass".to_string(),
        )
    };

    // Charts
    let mut dbh_hist_vals: Vec<f64> = Vec::new();
    for i in 0..df.height() {
        if let Some(d) = col_f64(df, "DBH", i) {
            dbh_hist_vals.push(d);
        }
    }
    let dbh_hist_svg = hist_svg(&dbh_hist_vals, "DBH distribution (cm)", 20, 640, 280);
    let growth_hist_svg = if is_multi && !growth_deltas.is_empty() {
        Some(hist_svg(&growth_deltas, "DBH growth Δ (cm)", 20, 640, 280))
    } else {
        None
    };

    // Limit bar charts to first ~40 bars for readability
    let mut ba_sorted = ba.clone();
    ba_sorted.sort_by(|a, b| {
        a.plot_id
            .cmp(&b.plot_id)
            .then(a.yr.partial_cmp(&b.yr).unwrap_or(std::cmp::Ordering::Equal))
    });
    ba_sorted.truncate(40);
    let ba_labels: Vec<String> = ba_sorted
        .iter()
        .map(|r| format!("{}-{}", r.plot_id, r.yr.floor() as i64))
        .collect();
    let ba_vals: Vec<f64> = ba_sorted.iter().map(|r| r.ba).collect();
    let ba_cols: Vec<String> = ba_sorted
        .iter()
        .map(|r| match r.ba_flag.as_str() {
            "critical" => "#E84855".into(),
            "warning" => "#F5A623".into(),
            _ => "#4DAF7C".into(),
        })
        .collect();
    let ba_empty_msg = ba_skip_reason.unwrap_or("no data");
    let ba_bar_svg = if ba_sorted.is_empty() {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="80"><text x="12" y="28" fill="#5f7068" font-family="system-ui,sans-serif" font-size="13">Basal area — {msg}</text></svg>"##,
            msg = ba_empty_msg
        )
    } else {
        bar_svg(
            &ba_labels,
            &ba_vals,
            &ba_cols,
            "Basal area by plot × census",
            "BA (m²/ha)",
            720,
            320,
            &[(50.0, "#F5A623"), (100.0, "#E84855")],
        )
    };

    let mut tph_sorted = tph.clone();
    tph_sorted.sort_by(|a, b| {
        a.plot_id
            .cmp(&b.plot_id)
            .then(a.yr.partial_cmp(&b.yr).unwrap_or(std::cmp::Ordering::Equal))
    });
    tph_sorted.truncate(40);
    let tph_labels: Vec<String> = tph_sorted
        .iter()
        .map(|r| format!("{}-{}", r.plot_id, r.yr.floor() as i64))
        .collect();
    let tph_vals: Vec<f64> = tph_sorted.iter().map(|r| r.tph).collect();
    let tph_cols: Vec<String> = vec!["#4DAF7C".into(); tph_vals.len()];
    let tph_bar_svg = if tph_sorted.is_empty() {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="80"><text x="12" y="28" fill="#5f7068" font-family="system-ui,sans-serif" font-size="13">TPH — {msg}</text></svg>"##,
            msg = ba_empty_msg
        )
    } else {
        bar_svg(
            &tph_labels,
            &tph_vals,
            &tph_cols,
            "Trees per hectare by plot × census",
            "TPH",
            720,
            320,
            &[],
        )
    };

    let charts = DiagnosticCharts {
        dbh_hist_svg,
        growth_hist_svg,
        ba_bar_svg,
        tph_bar_svg,
    };

    let mut report = DiagnosticReport {
        dataset_name: dataset_name.to_string(),
        census_type: if is_multi { "multi".into() } else { "single".into() },
        n_rows,
        n_trees,
        n_plots,
        yr_min,
        yr_max,
        status,
        flags,
        ba,
        tph,
        na_pa,
        na_species,
        unidentified,
        dbh,
        growth,
        charts,
        verdict,
        verdict_level,
        curation_log: curation_log.map(|s| s.to_string()),
        html: String::new(),
    };
    report.html = render_report_html(&report);
    Ok(report)
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_report_html(r: &DiagnosticReport) -> String {
    let title = if r.census_type == "multi" {
        "GFB3 Format Diagnostic Report"
    } else {
        "GFB2 / Single-census Diagnostic Report"
    };
    let yr = match (r.yr_min, r.yr_max) {
        (Some(a), Some(b)) => format!("{a:.0} – {b:.0}"),
        _ => "—".into(),
    };
    let verdict_cls = match r.verdict_level.as_str() {
        "fail" => "diag-fail",
        "warn" => "diag-warn",
        _ => "diag-pass",
    };

    let mut status_rows = String::new();
    for s in &r.status {
        status_rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td></tr>",
            esc_html(&s.status),
            esc_html(&s.label),
            s.n,
            s.pct
        ));
    }

    let mut flag_rows = String::new();
    for f in &r.flags {
        flag_rows.push_str(&format!(
            "<tr class=\"sev-{}\"><td>{}</td><td>{:.2}</td><td>{}</td></tr>",
            esc_html(&f.severity),
            esc_html(&f.flag),
            f.count,
            esc_html(&f.severity)
        ));
    }

    let growth_html = if let Some(g) = &r.growth {
        let chart = r.charts.growth_hist_svg.clone().unwrap_or_default();
        format!(
            r#"<h3>Growth summary</h3>
            <ul>
              <li>Paired alive stems: {}</li>
              <li>Mean ΔDBH: {}</li>
              <li>Mean annual growth: {}</li>
              <li>Negative growth: {}%</li>
              <li>Zero growth: {}</li>
              <li>Annual &gt; 5 cm/yr: {}</li>
            </ul>
            {}"#,
            g.n,
            g.mean_delta.map(|x| format!("{x:.3} cm")).unwrap_or_else(|| "—".into()),
            g.mean_annual
                .map(|x| format!("{x:.3} cm/yr"))
                .unwrap_or_else(|| "—".into()),
            g.pct_negative,
            g.n_zero,
            g.n_fast,
            chart
        )
    } else {
        "<h3>Growth summary</h3><p class=\"muted\">Not applicable for single-census (GFB2) data.</p>"
            .into()
    };

    let curation = r
        .curation_log
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            format!(
                "<h3>Curation notes</h3><pre class=\"curation\">{}</pre>",
                esc_html(s)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<div class="diag-report">
  <h2>{title}</h2>
  <div class="diag-verdict {verdict_cls}">{verdict}</div>
  <h3>Overview</h3>
  <ul>
    <li>Dataset: <code>{dsn}</code></li>
    <li>Mode: {mode}</li>
    <li>Rows: {rows}</li>
    <li>Trees: {trees}</li>
    <li>Plots: {plots}</li>
    <li>Year range: {yr}</li>
  </ul>
  {curation}
  <h3>Status distribution</h3>
  <table class="diag-table"><thead><tr><th>Status</th><th>Label</th><th>n</th><th>%</th></tr></thead>
  <tbody>{status_rows}</tbody></table>
  <h3>DBH summary</h3>
  <ul>
    <li>n: {dn}</li>
    <li>mean ± sd: {dmean} ± {dsd} cm</li>
    <li>min / Q25 / median / Q75 / max: {dmin} / {dq25} / {dmed} / {dq75} / {dmax}</li>
  </ul>
  {dbh_chart}
  {growth_html}
  <h3>Basal area</h3>
  {ba_chart}
  <h3>Trees per hectare</h3>
  {tph_chart}
  <h3>Data quality flags</h3>
  <table class="diag-table"><thead><tr><th>Flag</th><th>Count</th><th>Severity</th></tr></thead>
  <tbody>{flag_rows}</tbody></table>
</div>"#,
        title = title,
        verdict_cls = verdict_cls,
        verdict = esc_html(&r.verdict),
        dsn = esc_html(&r.dataset_name),
        mode = if r.census_type == "multi" {
            "Multi-census (GFB3)"
        } else {
            "Single-census (GFB2)"
        },
        rows = r.n_rows,
        trees = r.n_trees,
        plots = r.n_plots,
        yr = yr,
        curation = curation,
        status_rows = status_rows,
        dn = r.dbh.n,
        dmean = r.dbh.mean.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dsd = r.dbh.sd.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dmin = r.dbh.min.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dq25 = r.dbh.q25.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dmed = r.dbh.median.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dq75 = r.dbh.q75.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dmax = r.dbh.max.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        dbh_chart = &r.charts.dbh_hist_svg,
        growth_html = growth_html,
        ba_chart = &r.charts.ba_bar_svg,
        tph_chart = &r.charts.tph_bar_svg,
        flag_rows = flag_rows,
    )
}

/// Write a printable HTML report (open in browser → Print → PDF) and a plain-text summary PDF alternative.
pub fn write_diagnostic_html(report: &DiagnosticReport, path: &std::path::Path) -> std::io::Result<()> {
    let css = r#"
      body { font-family: system-ui,Segoe UI,sans-serif; color:#1a1d1b; margin:2rem; max-width:900px; }
      h2 { color:#1b4332; } h3 { color:#2d6a4f; margin-top:1.5rem; }
      .diag-verdict { padding:.75rem 1rem; border-radius:6px; font-weight:600; margin:1rem 0; }
      .diag-pass { background:#d8f3dc; color:#1b4332; }
      .diag-warn { background:#fef3c7; color:#92400e; }
      .diag-fail { background:#fee2e2; color:#b91c1c; }
      .diag-table { border-collapse:collapse; width:100%; font-size:13px; margin:.5rem 0 1rem; }
      .diag-table th,.diag-table td { border:1px solid #d0d5d2; padding:.35rem .5rem; text-align:left; }
      .diag-table th { background:#f4f6f5; }
      .sev-critical td { background:#fee2e2; }
      .sev-warning td { background:#fef3c7; }
      .muted { color:#5f7068; }
      pre.curation { white-space:pre-wrap; background:#f4f6f5; padding:1rem; border-radius:6px; }
      svg { max-width:100%; height:auto; display:block; margin:.75rem 0; }
    "#;
    let doc = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"/><title>{}</title><style>{}</style></head><body>{}</body></html>",
        esc_html(&report.dataset_name),
        css,
        report.html
    );
    std::fs::write(path, doc)
}

/// Write a simple text-based PDF of the diagnostic summary (tables + verdict).
pub fn write_diagnostic_pdf(report: &DiagnosticReport, path: &std::path::Path) -> Result<(), String> {
    use printpdf::*;
    use std::io::BufWriter;

    let (doc, page1, layer1) = PdfDocument::new(
        format!("Diagnostic - {}", report.dataset_name),
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_b = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let mut pages: Vec<(PdfPageIndex, PdfLayerIndex)> = vec![(page1, layer1)];
    let mut y = 280.0_f32;

    let mut lines: Vec<(bool, String)> = Vec::new();
    lines.push((
        true,
        if report.census_type == "multi" {
            "GFB3 Format Diagnostic Report".into()
        } else {
            "GFB2 / Single-census Diagnostic Report".into()
        },
    ));
    lines.push((false, format!("Dataset: {}", report.dataset_name)));
    lines.push((
        false,
        format!(
            "Mode: {}",
            if report.census_type == "multi" {
                "Multi-census (GFB3)"
            } else {
                "Single-census (GFB2)"
            }
        ),
    ));
    lines.push((
        false,
        format!(
            "Rows: {}   Trees: {}   Plots: {}",
            report.n_rows, report.n_trees, report.n_plots
        ),
    ));
    lines.push((
        false,
        format!(
            "Year range: {} - {}",
            report
                .yr_min
                .map(|x| format!("{x:.0}"))
                .unwrap_or_else(|| "-".into()),
            report
                .yr_max
                .map(|x| format!("{x:.0}"))
                .unwrap_or_else(|| "-".into())
        ),
    ));
    lines.push((false, String::new()));
    lines.push((true, format!("VERDICT: {}", report.verdict)));
    lines.push((false, String::new()));
    lines.push((true, "Status distribution:".into()));
    for s in &report.status {
        lines.push((
            false,
            format!("  {} ({})  n={}  {}%", s.status, s.label, s.n, s.pct),
        ));
    }
    lines.push((false, String::new()));
    lines.push((true, "Data quality flags:".into()));
    for f in &report.flags {
        lines.push((
            false,
            format!("  [{:>8}] {:>8.2}  {}", f.severity, f.count, f.flag),
        ));
    }
    lines.push((false, String::new()));
    lines.push((
        false,
        format!(
            "DBH: n={} mean={:?} median={:?}",
            report.dbh.n, report.dbh.mean, report.dbh.median
        ),
    ));
    if let Some(g) = &report.growth {
        lines.push((
            false,
            format!(
                "Growth: n={} mean_delta={:?} pct_neg={} fast={}",
                g.n, g.mean_delta, g.pct_negative, g.n_fast
            ),
        ));
    }
    lines.push((false, String::new()));
    lines.push((
        true,
        format!(
            "BA rows: {} (warning={}, critical={})",
            report.ba.len(),
            report.ba.iter().filter(|b| b.ba_flag == "warning").count(),
            report.ba.iter().filter(|b| b.ba_flag == "critical").count()
        ),
    ));
    for b in report.ba.iter().take(40) {
        lines.push((
            false,
            format!(
                "  {}  YR={:.0}  BA={:.3}  [{}]",
                b.plot_id, b.yr, b.ba, b.ba_flag
            ),
        ));
    }

    for (bold, line) in lines {
        if y < 20.0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer");
            pages.push((p, l));
            y = 280.0;
        }
        let (page, layer) = *pages.last().unwrap();
        let layer_ref = doc.get_page(page).get_layer(layer);
        let safe: String = line
            .chars()
            .map(|c| if c.is_ascii() { c } else { '?' })
            .collect();
        let f = if bold { &font_b } else { &font };
        let size = if bold && safe.starts_with("GFB") {
            14.0
        } else if bold {
            11.0
        } else {
            9.5
        };
        layer_ref.use_text(&safe, size, Mm(15.0), Mm(y), f);
        y -= if bold { 6.0 } else { 4.5 };
    }

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| e.to_string())
}

