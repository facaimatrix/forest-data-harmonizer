# GFB3 Harmonizer — Design

## Purpose
A contributor-facing desktop tool that absorbs the heterogeneity of incoming
forest-inventory data and guides a non-curator through producing a **draft** GFB3
paired-census dataset. The authoritative GFB3 finalization remains a curator
responsibility on the same `gfb3-core` engine (separate "curator" build).

## Why this split
- All variation across contributors is on the **input** side (wide vs long,
  idiosyncratic column names, different status vocabularies, different census
  structures, units). The GFB3 **output** is fixed.
- So the app is fundamentally a **guided schema-mapping + validation tool**. The
  flexibility lives in a per-contributor mapping config the user builds, not in
  hard-coded branching.
- Correctness-critical, judgment-laden steps stay single-sourced in `gfb3-core`
  and, where they require PI decisions, are escalated to the log rather than
  resolved by the contributor.

## Input gate: structural contract (require), not semantic schema (don't)
Require up front, because the app genuinely cannot guide around them:
- one rectangular table per dataset
- a single header row
- consistent row grain
- no merged cells, no stacked sub-tables / report-style layout

Do **not** require (these are the app's job, resolved in the flow):
- what columns mean, the status vocabulary, wide-vs-long shape, plot/census
  identity, units.

The gate runs after loading any file and, on failure, returns plain-language
diagnostics ("this sheet has merged cells and looks like two tables — please give
one clean table per dataset"), not a blind precondition.

## Guided prompt sequence
0. **Load file** (XLSX/CSV/PARQUET) → table preview → structural-contract gate.
1. **Dataset-level declarations the table often omits** — plots covered, census
   years, DBH units (cm vs mm), coordinate CRS, country/site. The contributor
   uniquely knows these and they are invisible in the raw numbers.
2. **Column mapping** — source columns → GFB3 fields, fuzzy auto-suggested from
   headers. Save the mapping keyed to the contributor's `gfb3_dsn` so repeat
   submissions reuse it.
3. **Shape** — detect wide format; offer a pivot/melt wizard if per-census
   year-columns exist ("tell me which columns are per-census DBH; I'll reshape").
4. **Status vocabulary → GFB3 status** — contributor maps their codes
   (alive/dead/recruit/"chopped down"/n-a) to GFB3 status, with conservative
   defaults wired in (ambiguous → 9; anthropogenic removal → 1 with note kept).
5. **Validation** — run the GFB3 checks (see below); show flagged rows in plain
   language for the contributor to resolve or escalate to the log.
6. **Export** — all three formats, `_DRAFT` suffix + embedded provenance, plus a
   metadata sidecar and a curation-log skeleton.

## Validation checks (port of report_gfb3 semantics)
- Duplicate TreeID within Plot + Year (dedup must run before any lag).
- First-census anchor detection (`PrevYR` null AND `Status == "0"`) vs recruits
  (`Status == "2"`, identified by `YR != min(YR)` per tree).
- Dead trees must have `DBH = null`.
- `Status == "2"` with no DBH → drop.
- `PrevYR` null AND `Status == "1"` → drop.
- Lag/previous-value structure correct only after sort by (PlotID, TreeID, YR)
  and group by both PlotID and TreeID.

## Provenance / draft marking
- Filename suffix `_DRAFT` for human legibility.
- Embedded, machine-verifiable provenance inside each file:
  - PARQUET: footer key-value metadata
  - XLSX: a metadata/properties sheet
  - CSV: a leading header comment line
  - Fields: `build=draft`, app_version, schema_version, contributor `gfb3_dsn`,
    timestamp.
- Because both apps compile from `gfb3-core`, the contributor build is configured
  so it cannot emit an authoritative stamp. (Optional future hardening: curator
  export carries a signature the draft build cannot generate.)

## Curation-log skeleton (fixed template — emit pre-filled where possible)
```
DATASET:
COUNTRY:
SITE:
PI:
CURATOR: Francisco Rivas
DATE RECEIVED:
DATE PROCESSED:
--- SOURCE FORMAT ---
--- PIVOT / RESTRUCTURING ---
--- DUPLICATE RESOLUTION ---
--- MISSING / INTERPOLATED DATA ---
--- SPECIES ISSUES ---
--- EXCLUSIONS ---
--- NOTES ---
```

## Anti-drift principle
`gfb3-core` is the single source of truth. The contributor and curator apps are
thin Tauri shells over it; draft-vs-authoritative is a runtime/build flag into the
same export path, never a parallel reimplementation.

## Open question
No persistent TreeID across censuses → cannot build paired-census linkage. Decide
flag-and-escalate vs hard stop; this sets the strictness of the input gate.
