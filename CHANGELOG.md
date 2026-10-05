# Changelog

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
