//! GFB3 → GFB2 export transform.
//!
//! GFB2 is the GFB3 observation table without Status and Prev* lag columns
//! (and without leftover source columns).

use polars::prelude::*;

use crate::schema::{gfb2_export_columns, select_export_columns, with_plot_yr, STATUS_ALIVE};

/// Convert a cleaned GFB3 frame to GFB2.
///
/// - Optionally keeps only Status == "0" (alive) rows
/// - Projects to the canonical GFB2 schema only (no Status, no Prev*, no
///   leftover source columns)
/// - Deduplicates on (PlotID, TreeID, YR)
pub fn gfb3_to_gfb2(lf: LazyFrame, keep_alive_only: bool) -> PolarsResult<DataFrame> {
    let lf = if keep_alive_only {
        lf.filter(col("Status").cast(DataType::String).eq(lit(STATUS_ALIVE)))
    } else {
        lf
    };

    let df = lf.collect()?;
    let df = with_plot_yr(df)?;
    let df = select_export_columns(df, gfb2_export_columns())?;

    df.unique_stable(
        Some(&["PlotID".into(), "TreeID".into(), "YR".into()]),
        UniqueKeepStrategy::First,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gfb3() -> DataFrame {
        DataFrame::new(vec![
            Column::from(Series::new("PlotID".into(), &["P1", "P1", "P1"])),
            Column::from(Series::new("TreeID".into(), &["T1", "T1", "T2"])),
            Column::from(Series::new("YR".into(), &[2010u32, 2015, 2015])),
            Column::from(Series::new(
                "PrevYR".into(),
                &[None, Some(2010u32), None],
            )),
            Column::from(Series::new("Status".into(), &["0", "0", "1"])),
            Column::from(Series::new(
                "DBH".into(),
                &[Some(10.0f64), Some(11.0), None],
            )),
            Column::from(Series::new("PA".into(), &[1.0f64, 1.0, 1.0])),
            Column::from(Series::new(
                "Latitude".into(),
                &[Some(-1.0f64), Some(-1.0), Some(-1.0)],
            )),
            Column::from(Series::new(
                "Longitude".into(),
                &[Some(30.0f64), Some(30.0), Some(30.0)],
            )),
            Column::from(Series::new(
                "Species".into(),
                &["sp_a", "sp_a", "sp_b"],
            )),
        ])
        .unwrap()
    }

    #[test]
    fn gfb2_keeps_alive_only_and_drops_lags() {
        let out = gfb3_to_gfb2(sample_gfb3().lazy(), true).unwrap();
        assert_eq!(out.height(), 2);
        let names: Vec<_> = out
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!names.iter().any(|n| n == "PrevYR"));
        assert!(!names.iter().any(|n| n == "Status"));
        assert_eq!(
            names,
            vec![
                "PlotID",
                "TreeID",
                "YR",
                "PlotYR",
                "DBH",
                "Species",
                "Latitude",
                "Longitude",
                "PA",
            ]
        );
    }

    #[test]
    fn gfb2_includes_expan_when_present() {
        use crate::schema::{with_expan, ExpanSpec};
        let df = with_expan(sample_gfb3(), ExpanSpec::FixedArea).unwrap();
        let out = gfb3_to_gfb2(df.lazy(), true).unwrap();
        let names: Vec<_> = out
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(names.iter().any(|n| n == "EXPAN"));
        let expan = out.column("EXPAN").unwrap().f64().unwrap();
        assert!((expan.get(0).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gfb2_strips_source_columns() {
        let mut df = sample_gfb3();
        df.with_column(Series::new("raw_plot_code".into(), &["a", "a", "b"]))
            .unwrap();
        df.with_column(Series::new("notes".into(), &["x", "y", "z"]))
            .unwrap();
        let out = gfb3_to_gfb2(df.lazy(), true).unwrap();
        let names: Vec<_> = out
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!names.iter().any(|n| n == "raw_plot_code"));
        assert!(!names.iter().any(|n| n == "notes"));
    }
}
