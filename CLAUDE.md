# CLAUDE.md — GFB3 Harmonizer

## What this is
A standalone **Rust + Tauri v2** desktop app that lets forest-inventory data
**contributors** wrangle their own datasets into the GFB3 paired-census format
through a guided set of prompts, then export a **draft** GFB3 dataset (XLSX, CSV,
PARQUET). It is part of the broader migration of the FACAI / GFB3 tooling from R
to Rust. The authoritative finalization stays with the curator (separate build).

Curator of record (used in all curation logs): **Francisco Rivas**.

## Architecture (decided)
- **One core crate, two thin shells.** All schema, validation, pivot/transform,
  and log-generation logic lives in a single pure-Rust crate `gfb3-core`
  (Polars-based). The contributor app and the future curator app are thin Tauri
  v2 front-ends over that crate. *Do not fork logic between apps* — the
  draft-vs-authoritative difference is a flag into the same export function.
- **Draft provenance is embedded, not just suffixed.** Exports get a filename
  suffix (e.g. `_DRAFT`) for humans AND machine-verifiable provenance inside the
  file: parquet footer key-value metadata, an XLSX metadata/properties sheet, a
  CSV header comment line. Record: `build=draft`, app version, schema version,
  contributor `gfb3_dsn`, timestamp.
- **Input gate = structural contract, not semantic schema.** Require only: one
  rectangular table per dataset, single header row, consistent row grain, no
  merged cells, no stacked sub-tables. Everything semantic (column meaning,
  status vocabulary, wide-vs-long, plot/census identity, units) is resolved
  *inside* the guided flow. The gate fails with plain-language diagnostics, never
  a precondition the user must satisfy blind.

## Suggested workspace layout
```
gfb3-harmonizer/
  Cargo.toml              # cargo workspace
  crates/
    gfb3-core/            # pure Rust lib: schema, validation, transform, logs
  apps/
    contributor/          # Tauri v2 app (draft build)  <-- build first
    curator/              # Tauri v2 app (authoritative build) <-- later
  docs/
    DESIGN.md
```

## Tech stack
- Rust, Tauri v2 (already the team's chosen desktop stack)
- **Polars** for all dataframe work (arrow-native; matches the parquet pipeline)
- **calamine** for reading XLSX/XLS; arrow/parquet for parquet I/O; csv crate
- Frontend: whatever the contributor app uses for the wizard UI (table preview,
  column-mapping dropdowns, status-vocabulary editor)

## GFB3 domain invariants (gfb3-core MUST enforce these)
These are hard-won rules carried over from the R `facai` pipeline. Express them
in Polars, not R idioms.
- **Status is a string the entire pipeline; cast to integer ONLY at export**
  (the `coerce` step).
- **Drop first-census anchor rows before export**: rows where `PrevYR` is null
  AND `Status == "0"`. Recruits (`Status == "2"`) are distinguished from anchors
  by `YR != min(YR)` per tree — NOT by the null pattern alone.
- **Any lag / previous-value computation** requires sorting by
  `(PlotID, TreeID, YR)` and grouping by **both** `PlotID` and `TreeID`.
- **Deduplicate before computing any lag.**
- **Dataset-specific status remaps** (e.g. `"n/a" -> "1"`) are explicit in the
  per-dataset mapping config; never buried inside the coerce step.
- **Ambiguous / unknown status -> `"9"` (missing).** Never silently drop or
  impute without writing to the log.
- **Dead trees have `DBH = null`.** Rows with `PrevYR` null AND `Status == "1"`
  are dropped. `Status == "2"` with no DBH is dropped.
- **When binding multiple files** with non-GFB3 columns, cast all non-GFB3
  columns to string.

## Curation philosophy (encode in UI defaults + log behavior)
- Recode ambiguous cases to `9` rather than impute.
- Flag anthropogenic removals ("chopped down") as `Status == "1"` with the
  original note preserved.
- Escalate PI-level judgment calls to the curation log instead of resolving them
  in-app. The contributor app especially must NOT make these calls.
- Verify data before applying a fix; rebuild intermediate objects after any
  status/ID change (stale state is a common bug source).

## What the app outputs
For each dataset: the GFB3 draft in all three formats (suffixed + provenance-
stamped), a partially-filled metadata sidecar, and a **curation-log skeleton** in
the fixed template (see docs/DESIGN.md).

## Open question to resolve early
Contributors who lack persistent TreeIDs across censuses: the app can detect the
absence but cannot fabricate paired-census linkage. Decide whether this is a
flag-and-escalate or a hard stop — it sets how strict the input gate is.
