# Changelog

## 0.2.0 — 2026-10-06

- Keep `:` in instrument and RNS identifiers in request paths. Ticker.app
  answers `/prices/XLON%3ALLOY` with 404; the identifier parameters are marked
  `allowReserved` and generated with `openapi-to-rust` 0.22.0.
- Decode the live price snapshot: 52-week high and low dates are timestamps
  (`DateTime<Utc>`, was `NaiveDate`) and the 52-week average volume is an
  `f64` (was `i64`).
- Add the `RNS` category kind.
- Add GitHub Actions workflows and tag-based releasing with `bump` and `tag` tasks.

## 0.1.0 — 2026-10-03

- Generate all 14 operations from the pinned Ticker Data API 2.2.1 specification.
- Preserve the additional `latestCursor` field observed in live disclosure pagination
  through a documented overlay; keep the supplied specification unchanged.
- Add convenience methods for snapshots, timeseries, disclosures and exchanges.
- Preserve missing prices, provider warnings, timestamps and pagination metadata.
- Add bounded history collection with explicit errors for incomplete pagination,
  repeated cursors and invalid keyed dates.
- Add reproducible generation, tests and tag-based release tasks through mise.
- Require Rust 1.98.1 and add GitHub Actions lint, test and release dry-run workflows.
