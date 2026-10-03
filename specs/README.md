# Specification provenance

`tickerapp.openapi.yaml` is the exact supplied specification snapshot, without
in-place edits. It declares OpenAPI 3.0.1, title **Ticker Data API**, API version
**2.2.1**, and server `https://api.tickerapp.net/v2`. It contains 14 GET operations,
including an internal health operation. It is not automatically refreshed from
the provider's developer portal.

Snapshot SHA-256:
`fbc7abfe1a2b10485f518fd420def85591f86fc61b59fd13ace3000a39010dd3`.

The sibling `tickerapp.overlay.yaml` and generator configuration carry forward
compatibility choices from the existing integration:

- Remove `required` lists within `PriceSnapshot`, including its nested objects,
  to accept partial or empty snapshots. Missing values remain optional; a valid
  snapshot does not imply a usable price. History's OHLCV requirements stay intact.
- Make `DisclosureItem.version` nullable.
- Retain unknown quote-change directions, category kinds and category importance
  values as `Custom(String)` rather than failing the whole response.

A further overlay adds optional `CursorPaging.latestCursor`. A live disclosure
response on 2026-10-03 included this field, which the supplied snapshot does not
define. Without it, the generator's lossless `oneOf` decoder rejects the paging
object. The client retains the opaque cursor; it does not interpret it or use it
as a replacement for `nextCursor`. A mock regression test covers this shape.

These are client compatibility decisions, not edits claimed to come from the
provider. `src/generated/effective.json` records the effective schema.

`openapi-to-rust.toml` generates all operations with openapi-to-rust 0.19.0,
header authentication, no request tracing and a bounded response body. No app
models, instrument mappings, notification policy or scheduler are included.

The supplied specification has no `info.license` declaration. Its provenance
is preserved separately from the licence of the handwritten client code.

[Provider developer portal](https://developers.ticker.app/)
