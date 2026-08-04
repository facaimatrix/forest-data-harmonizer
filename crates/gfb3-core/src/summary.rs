//! Plot-level and dataset-level summary tables for export.
//!
//! Replaces the former FISI product. Metrics follow the same alive-stem /
//! PA conventions as the diagnostic BA/TPH calculations.

use polars::prelude::*;
use std::collections::BTreeMap;

use crate::mapping::{CensusType, DatasetMetadata, DbhUnit};
use crate::schema::{STATUS_ALIVE, STATUS_DEAD, STATUS_MISSING, STATUS_RECRUIT};

const DBH_BIN_CM: f64 = 5.0;

fn has_col(df: &DataFrame, name: &str) -> bool {
    df.get_column_names().iter().any(|c| c.as_str() == name)
}

fn col_str(df: &DataFrame, name: &str, i: usize) -> String {
    let Ok(col) = df.column(name) else {
        return String::new();
    };
    match col.get(i).unwrap_or(AnyValue::Null) {
        AnyValue::Null => String::new(),
        AnyValue::String(s) => s.to_string(),
        AnyValue::StringOwned(s) => s.to_string(),
        other => format!("{other}"),
    }
}

fn col_f64(df: &DataFrame, name: &str, i: usize) -> Option<f64> {
    let Ok(col) = df.column(name) else {
        return None;
    };
    match col.get(i).unwrap_or(AnyValue::Null) {
        AnyValue::Null => None,
        AnyValue::Float64(x) => Some(x),
        AnyValue::Float32(x) => Some(f64::from(x)),
        AnyValue::Int64(x) => Some(x as f64),
        AnyValue::Int32(x) => Some(f64::from(x)),
        AnyValue::UInt32(x) => Some(f64::from(x)),
        AnyValue::UInt64(x) => Some(x as f64),
        AnyValue::String(s) => s.trim().parse().ok(),
        AnyValue::StringOwned(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn status_at(df: &DataFrame, i: usize) -> String {
    if !has_col(df, "Status") {
        return STATUS_ALIVE.to_string();
    }
    let s = col_str(df, "Status", i);
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("null") {
        STATUS_ALIVE.to_string()
    } else {
        t.to_string()
    }
}

fn is_alive(st: &str) -> bool {
    st == STATUS_ALIVE
}

fn shannon(counts: &BTreeMap<String, u64>) -> Option<f64> {
    let total: u64 = counts.values().sum();
    if total == 0 {
        return None;
    }
    let n = total as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        if c == 0 {
            continue;
        }
        let p = c as f64 / n;
        h -= p * p.ln();
    }
    Some(h)
}

/// Gini–Simpson index: 1 − Σ pᵢ²
fn simpson(counts: &BTreeMap<String, u64>) -> Option<f64> {
    let total: u64 = counts.values().sum();
    if total == 0 {
        return None;
    }
    let n = total as f64;
    let mut sum_sq = 0.0;
    for &c in counts.values() {
        let p = c as f64 / n;
        sum_sq += p * p;
    }
    Some(1.0 - sum_sq)
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[derive(Default)]
struct PlotAgg {
    lat: Option<f64>,
    lon: Option<f64>,
    pa: Option<f64>,
    ba_sum_m2: f64,
    n_alive_dbh: u64,
    dbh_sum: f64,
    dbh_n: u64,
    species: BTreeMap<String, u64>,
    dbh_bins: BTreeMap<String, u64>,
    n_status_0: u64,
    n_status_1: u64,
    n_status_2: u64,
    n_status_9: u64,
    n_status_other: u64,
}

fn accumulate_row(df: &DataFrame, i: usize, agg: &mut PlotAgg) {
    if has_col(df, "Latitude") {
        if let Some(v) = col_f64(df, "Latitude", i) {
            if agg.lat.is_none() {
                agg.lat = Some(v);
            }
        }
    }
    if has_col(df, "Longitude") {
        if let Some(v) = col_f64(df, "Longitude", i) {
            if agg.lon.is_none() {
                agg.lon = Some(v);
            }
        }
    }
    if has_col(df, "PA") {
        if let Some(v) = col_f64(df, "PA", i) {
            if v > 0.0 {
                agg.pa = Some(v);
            }
        }
    }

    let st = status_at(df, i);
    match st.as_str() {
        s if s == STATUS_ALIVE => agg.n_status_0 += 1,
        s if s == STATUS_DEAD => agg.n_status_1 += 1,
        s if s == STATUS_RECRUIT => agg.n_status_2 += 1,
        s if s == STATUS_MISSING => agg.n_status_9 += 1,
        _ => agg.n_status_other += 1,
    }

    if !is_alive(&st) {
        return;
    }
    let Some(dbh) = col_f64(df, "DBH", i) else {
        return;
    };
    if !dbh.is_finite() || dbh < 0.0 {
        return;
    }

    agg.n_alive_dbh += 1;
    agg.dbh_sum += dbh;
    agg.dbh_n += 1;
    agg.ba_sum_m2 += std::f64::consts::PI * (dbh / 200.0).powi(2);

    if has_col(df, "Species") {
        let sp = col_str(df, "Species", i);
        let sp = sp.trim();
        if !sp.is_empty() && !sp.eq_ignore_ascii_case("null") {
            *agg.species.entry(sp.to_string()).or_insert(0) += 1;
        }
    }

    let bin = (dbh / DBH_BIN_CM).floor() as i64;
    let bin_key = format!("{bin}");
    *agg.dbh_bins.entry(bin_key).or_insert(0) += 1;
}

/// One row per PlotID × YR (census measurement), with PlotYR = `{PlotID}_{YR}`.
pub fn build_plots_summary(df: &DataFrame) -> PolarsResult<DataFrame> {
    let mut by_key: BTreeMap<(String, i64), PlotAgg> = BTreeMap::new();
    for i in 0..df.height() {
        let plot = if has_col(df, "PlotID") {
            let p = col_str(df, "PlotID", i);
            if p.trim().is_empty() {
                continue;
            }
            p
        } else {
            continue;
        };
        let yr = if has_col(df, "YR") {
            col_f64(df, "YR", i).map(|y| y.floor() as i64).unwrap_or(0)
        } else {
            0
        };
        let agg = by_key.entry((plot, yr)).or_default();
        accumulate_row(df, i, agg);
    }

    let n = by_key.len();
    let mut plot_id = Vec::with_capacity(n);
    let mut yr_col = Vec::with_capacity(n);
    let mut plot_yr = Vec::with_capacity(n);
    let mut lat = Vec::with_capacity(n);
    let mut lon = Vec::with_capacity(n);
    let mut ba = Vec::with_capacity(n);
    let mut tph = Vec::with_capacity(n);
    let mut dbh_mean = Vec::with_capacity(n);
    let mut richness = Vec::with_capacity(n);
    let mut sp_shannon = Vec::with_capacity(n);
    let mut sp_simpson = Vec::with_capacity(n);
    let mut dbh_shannon = Vec::with_capacity(n);
    let mut dbh_simpson = Vec::with_capacity(n);
    let mut n0 = Vec::with_capacity(n);
    let mut n1 = Vec::with_capacity(n);
    let mut n2 = Vec::with_capacity(n);
    let mut n9 = Vec::with_capacity(n);

    for ((pid, yr), a) in by_key {
        plot_yr.push(format!("{pid}_{yr}"));
        plot_id.push(pid);
        yr_col.push(yr as u32);
        lat.push(a.lat);
        lon.push(a.lon);
        let (ba_v, tph_v) = match a.pa {
            Some(pa) if pa > 0.0 => (Some(round3(a.ba_sum_m2 / pa)), Some(round3(a.n_alive_dbh as f64 / pa))),
            _ => (None, None),
        };
        ba.push(ba_v);
        tph.push(tph_v);
        dbh_mean.push(if a.dbh_n > 0 {
            Some(round3(a.dbh_sum / a.dbh_n as f64))
        } else {
            None
        });
        richness.push(a.species.len() as u32);
        sp_shannon.push(shannon(&a.species).map(round3));
        sp_simpson.push(simpson(&a.species).map(round3));
        dbh_shannon.push(shannon(&a.dbh_bins).map(round3));
        dbh_simpson.push(simpson(&a.dbh_bins).map(round3));
        n0.push(a.n_status_0);
        n1.push(a.n_status_1);
        n2.push(a.n_status_2);
        n9.push(a.n_status_9);
    }

    DataFrame::new(vec![
        Column::from(Series::new("PlotID".into(), plot_id)),
        Column::from(Series::new("YR".into(), yr_col)),
        Column::from(Series::new("PlotYR".into(), plot_yr)),
        Column::from(Series::new("Latitude".into(), lat)),
        Column::from(Series::new("Longitude".into(), lon)),
        Column::from(Series::new("BA".into(), ba)),
        Column::from(Series::new("TPH".into(), tph)),
        Column::from(Series::new("DBH_mean".into(), dbh_mean)),
        Column::from(Series::new("species_richness".into(), richness)),
        Column::from(Series::new("species_shannon".into(), sp_shannon)),
        Column::from(Series::new("species_simpson".into(), sp_simpson)),
        Column::from(Series::new("dbh_shannon".into(), dbh_shannon)),
        Column::from(Series::new("dbh_simpson".into(), dbh_simpson)),
        Column::from(Series::new("n_status_0".into(), n0)),
        Column::from(Series::new("n_status_1".into(), n1)),
        Column::from(Series::new("n_status_2".into(), n2)),
        Column::from(Series::new("n_status_9".into(), n9)),
    ])
}

/// Single-row dataset summary (metadata + pooled metrics).
pub fn build_dataset_summary(df: &DataFrame, meta: &DatasetMetadata, dsn: &str) -> PolarsResult<DataFrame> {
    let mut all = PlotAgg::default();
    let mut plots: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut yrs: Vec<f64> = Vec::new();

    for i in 0..df.height() {
        if has_col(df, "PlotID") {
            let p = col_str(df, "PlotID", i);
            if !p.trim().is_empty() {
                plots.insert(p);
            }
        }
        if has_col(df, "YR") {
            if let Some(y) = col_f64(df, "YR", i) {
                yrs.push(y);
            }
        }
        accumulate_row(df, i, &mut all);
    }

    // Dataset BA/TPH: sum stem BA over sum of unique plot areas (one PA per plot).
    let n_plots = plots.len() as u32;
    let mut pa_sum = 0.0;
    let mut ba_sum = 0.0;
    let mut tph_num = 0.0;
    let mut tph_den = 0.0;
    let mut plot_pa: BTreeMap<String, f64> = BTreeMap::new();
    let mut plot_ba_m2: BTreeMap<String, f64> = BTreeMap::new();
    let mut plot_n_alive: BTreeMap<String, u64> = BTreeMap::new();
    for i in 0..df.height() {
        if !has_col(df, "PlotID") {
            break;
        }
        let pid = col_str(df, "PlotID", i);
        if pid.trim().is_empty() {
            continue;
        }
        if has_col(df, "PA") {
            if let Some(pa) = col_f64(df, "PA", i) {
                if pa > 0.0 {
                    plot_pa.insert(pid.clone(), pa);
                }
            }
        }
        let st = status_at(df, i);
        if !is_alive(&st) {
            continue;
        }
        let Some(dbh) = col_f64(df, "DBH", i) else { continue };
        if !dbh.is_finite() || dbh < 0.0 {
            continue;
        }
        *plot_ba_m2.entry(pid.clone()).or_insert(0.0) +=
            std::f64::consts::PI * (dbh / 200.0).powi(2);
        *plot_n_alive.entry(pid).or_insert(0) += 1;
    }
    for (pid, pa) in &plot_pa {
        pa_sum += pa;
        if let Some(ba_m2) = plot_ba_m2.get(pid) {
            ba_sum += ba_m2;
            tph_num += *plot_n_alive.get(pid).unwrap_or(&0) as f64;
            tph_den += pa;
        }
    }

    let ba = if pa_sum > 0.0 {
        Some(round3(ba_sum / pa_sum))
    } else {
        None
    };
    let tph = if tph_den > 0.0 {
        Some(round3(tph_num / tph_den))
    } else {
        None
    };
    let dbh_mean = if all.dbh_n > 0 {
        Some(round3(all.dbh_sum / all.dbh_n as f64))
    } else {
        None
    };

    let yr_min = yrs.iter().cloned().fold(None, |acc: Option<f64>, y| {
        Some(acc.map_or(y, |a| a.min(y)))
    });
    let yr_max = yrs.iter().cloned().fold(None, |acc: Option<f64>, y| {
        Some(acc.map_or(y, |a| a.max(y)))
    });

    let census_type = match meta.census_type {
        CensusType::Single => "single",
        CensusType::Multi => "multi",
    };
    let dbh_unit = match meta.dbh_unit {
        Some(DbhUnit::Mm) => Some("mm".to_string()),
        Some(DbhUnit::Cm) => Some("cm".to_string()),
        None => None,
    };
    let census_years = if meta.census_years.is_empty() {
        None
    } else {
        Some(
            meta.census_years
                .iter()
                .map(|y| y.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
    };

    DataFrame::new(vec![
        Column::from(Series::new("gfb3_dsn".into(), vec![dsn.to_string()])),
        Column::from(Series::new(
            "country".into(),
            vec![meta.country.clone()],
        )),
        Column::from(Series::new("site".into(), vec![meta.site.clone()])),
        Column::from(Series::new(
            "contact".into(),
            vec![meta.contact.clone()],
        )),
        Column::from(Series::new(
            "contact_email".into(),
            vec![meta.contact_email.clone()],
        )),
        Column::from(Series::new("pi".into(), vec![meta.pi.clone()])),
        Column::from(Series::new(
            "pi_email".into(),
            vec![meta.pi_email.clone()],
        )),
        Column::from(Series::new(
            "census_type".into(),
            vec![census_type.to_string()],
        )),
        Column::from(Series::new("census_years".into(), vec![census_years])),
        Column::from(Series::new("dbh_unit".into(), vec![dbh_unit])),
        Column::from(Series::new("n_plots".into(), vec![n_plots])),
        Column::from(Series::new("n_rows".into(), vec![df.height() as u32])),
        Column::from(Series::new("yr_min".into(), vec![yr_min])),
        Column::from(Series::new("yr_max".into(), vec![yr_max])),
        Column::from(Series::new("BA".into(), vec![ba])),
        Column::from(Series::new("TPH".into(), vec![tph])),
        Column::from(Series::new("DBH_mean".into(), vec![dbh_mean])),
        Column::from(Series::new(
            "species_richness".into(),
            vec![all.species.len() as u32],
        )),
        Column::from(Series::new(
            "species_shannon".into(),
            vec![shannon(&all.species).map(round3)],
        )),
        Column::from(Series::new(
            "species_simpson".into(),
            vec![simpson(&all.species).map(round3)],
        )),
        Column::from(Series::new(
            "dbh_shannon".into(),
            vec![shannon(&all.dbh_bins).map(round3)],
        )),
        Column::from(Series::new(
            "dbh_simpson".into(),
            vec![simpson(&all.dbh_bins).map(round3)],
        )),
        Column::from(Series::new("n_status_0".into(), vec![all.n_status_0])),
        Column::from(Series::new("n_status_1".into(), vec![all.n_status_1])),
        Column::from(Series::new("n_status_2".into(), vec![all.n_status_2])),
        Column::from(Series::new("n_status_9".into(), vec![all.n_status_9])),
        Column::from(Series::new(
            "coordinate_crs".into(),
            vec![meta.coordinate_crs.clone()],
        )),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DataFrame {
        DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1", "P1", "P2"])),
            Column::from(Series::new("TreeID".into(), &["T1", "T2", "T3"])),
            Column::from(Series::new("YR".into(), &[2010u32, 2010, 2010])),
            Column::from(Series::new("Status".into(), &["0", "0", "1"])),
            Column::from(Series::new(
                "DBH".into(),
                &[Some(10.0f64), Some(20.0), Some(15.0)],
            )),
            Column::from(Series::new("PA".into(), &[1.0f64, 1.0, 0.5])),
            Column::from(Series::new(
                "Latitude".into(),
                &[Some(-1.0f64), Some(-1.0), Some(-2.0)],
            )),
            Column::from(Series::new(
                "Longitude".into(),
                &[Some(30.0f64), Some(30.0), Some(31.0)],
            )),
            Column::from(Series::new(
                "Species".into(),
                &["sp_a", "sp_b", "sp_a"],
            )),
        ])
        .unwrap()
    }

    #[test]
    fn plots_summary_one_row_per_plot() {
        let out = build_plots_summary(&sample()).unwrap();
        assert_eq!(out.height(), 2);
        let names: Vec<_> = out
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(names.contains(&"BA".into()));
        assert!(names.contains(&"PlotYR".into()));
        assert!(names.contains(&"species_shannon".into()));
        assert!(names.contains(&"n_status_0".into()));
        let py: Vec<Option<&str>> = out
            .column("PlotYR")
            .unwrap()
            .str()
            .unwrap()
            .into_iter()
            .collect();
        assert!(py.iter().any(|v| v == &Some("P1_2010")));
    }

    #[test]
    fn dataset_summary_single_row() {
        let meta = DatasetMetadata {
            country: Some("PER".into()),
            site: Some("Manu".into()),
            pi: Some("Ada".into()),
            contact: Some("Grace Hopper".into()),
            contact_email: Some("g@example.com".into()),
            pi_email: None,
            dbh_unit: Some(DbhUnit::Cm),
            coordinate_crs: None,
            census_years: vec![2010],
            census_type: CensusType::Single,
        };
        let out = build_dataset_summary(&sample(), &meta, "in_per_hopper_2026_s").unwrap();
        assert_eq!(out.height(), 1);
        assert_eq!(
            out.column("n_plots").unwrap().u32().unwrap().get(0),
            Some(2)
        );
    }
}
