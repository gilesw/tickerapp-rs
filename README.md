# tickerapp

Unofficial async Rust client for the Ticker.app v2 market data API.
Generated from the checked-in OpenAPI 3.0.1 specification, API version **2.2.1**,
with a small convenience layer. Requires Rust 1.98.1 or newer.

This crate is being prepared for publication. For now, use a local dependency:

```toml
[dependencies]
tickerapp = { path = "../tickerapp-rs" }
```

After publication, use `tickerapp = "0.1"`.

## Prices and disclosures

```rust,no_run
use tickerapp::{Client, DisclosureQuery, PageQuery};

# async fn example() -> Result<(), tickerapp::Error> {
let client = Client::new("your-api-key");
let snapshot = client.price("XLON:LLOY").await?;
if let Some(close) = snapshot.session.and_then(|session| session.close) {
    println!("Session price: {close}");
}

let page = client.disclosures(&DisclosureQuery {
    symbols: vec!["LLOY".into()],
    page: PageQuery { page_size: Some(20), ..Default::default() },
    ..Default::default()
}).await?;
// Keep warnings visible: a plan may not support a requested filter.
println!("Warnings: {:?}", page.warnings);
for item in page.data {
    println!("{}: {}", item.timestamp, item.headline);
}
# Ok(())
# }
```

Identifiers accept ISIN or `MIC:SYMBOL` where supported by the provider. They are
encoded as individual URL path segments. Keys are sent in the `x-api-key` header,
not the URL. `Client` debug output omits credentials.

The example reads its key from standard input and requests one disclosure:

```sh
cargo run --example disclosures < /path/to/private-key-file
```

## Coverage

All **14 operations in the pinned specification** are available through
`client.raw()`. That includes the spec's internal health operation; it is not
promised as a supported public endpoint.

| Area | Convenience methods | Additional generated operations |
| --- | --- | --- |
| Prices | `price`, `price_with_change_basis` | — |
| History | `timeseries`, `all_timeseries` | — |
| Disclosures | `disclosures`, `disclosure` | — |
| Exchanges | `exchanges`, `exchange` | — |
| NAV and trades | — | `get_nav`, `get_trades` |
| Market statistics | — | Risers, fallers, highest volume, highest turnover, most traded |
| Health (internal) | — | `health` |

The generated module exposes typed query enums, response models and endpoint
errors. This is coverage of the supplied snapshot, not a claim that it includes
subsequent additions to the service. Endpoint access, delays and available
history depend on the provider and subscription plan.

## History and pagination

`timeseries`, `disclosures` and `exchanges` return one page with pagination metadata.
Use either `PageQuery.page_cursor` or `page_number`, not both. The convenience
layer rejects conflicting modes and non-positive page sizes/numbers before making
a request. Endpoint-specific maximum sizes are enforced by the provider.

For bounded cursor collection:

```rust,no_run
use tickerapp::{Client, TimeseriesQuery};

# async fn example() -> Result<(), tickerapp::Error> {
let client = Client::new("your-api-key");
let rows = client.all_timeseries("XLON:LLOY", &TimeseriesQuery {
    date_from: Some("2025-01-01".into()),
    date_to: Some("2025-01-31".into()),
    ..Default::default()
}, 20).await?;
# Ok(())
# }
```

The page limit counts requests. The helper returns `Error::Pagination` if more
history remains at the limit, the provider repeats a cursor, returns an empty
continuation cursor, or switches to classic paging. It never reports truncated
history as complete. Use the page method when incremental processing or resuming
from a saved cursor matters. Starting with a cursor collects only from that point.

`timeseries_items` normalizes arrays and ISO-date/Unix-seconds-keyed responses.
Invalid date keys are errors, not discarded rows. Array order is preserved;
keyed responses use key order. There is no sorting, deduplication, price adjustment
or unit conversion. Original dates, timestamps and numeric values are retained.

Missing snapshot values remain `Option` values. An empty snapshot can be returned
successfully; callers decide whether it contains the fields they need. Required
OHLCV values in history must be present. No unavailable price becomes zero.
Nullable optional generated fields may use `Option<Option<T>>` to distinguish
an omitted field from explicit JSON null.

A session's `close` field can represent a current session price. The client does
not decide that a trading session is complete or choose between mid, trade and
closing prices for an application's rules.

## Errors and configuration

Convenience methods return `tickerapp::Error`:

- `Api`: HTTP status, original response body and `Retry-After` header.
- `Decode`: a successful response that does not match the expected schema.
- `Transport`: request, transport or response-size failure.
- `InvalidRequest`: invalid pagination arguments.
- `Pagination`: bounded history collection could not finish.

`is_unauthorized()` recognizes HTTP 401/403; `is_rate_limited()` recognizes 429.
Raw operations return generated endpoint-specific errors, convertible into `Error`.
There are no automatic retries, caching, quota accounting or background tasks.

The default API root is `https://api.tickerapp.net/v2`, with a 30-second timeout
and an 8 MiB response-body limit. `with_base_url` accepts a complete API root,
including `/v2`; `with_max_response_body_bytes` changes the response limit.
For custom transport/timeouts, use the builders on `generated::HttpClient`.

## Development

Generated code is checked in. Building the crate does not require a generator
or a specification download.

```sh
mise run generate
mise run generate:check
mise run check
```

Generation pins `openapi-to-rust` 0.22.0, the first release that honours `allowReserved` on
path parameters, so `XLON:LLOY` keeps its colon, and installs it locally under `target/tools`.
Edit the specification/configuration/overlay, then regenerate; never hand-edit
`src/generated`. [Specification provenance and compatibility changes](specs/README.md)
records exactly what is generated. Tests use synthetic responses and local mock
servers; no credentials or account data are stored in fixtures.

Verification on 2026-10-06: 17 client tests, one documentation example and
47 release-script tests pass. Live requests on 2026-10-06 decoded a price
snapshot by `XLON:LLOY`, a timeseries page and an RNS item looked up by GUID;
RNS disclosure pages were checked on 2026-10-05. Exchanges were listed live;
NAV, trades and market statistics have not been live-verified. Formatting,
Clippy and generated-output checks also pass.

The [release guide](RELEASING.md) covers `mise run bump` for the version and
changelog, `mise run tag` for the annotated tag and push, `mise run release` for
a dry run and `mise run release:publish` for upload. Pushing the tag publishes
through crates.io trusted publishing; local uploads read an inherited
`CARGO_REGISTRY_TOKEN`.

## License and provenance

Handwritten code is MIT OR Apache-2.0. The upstream specification is retained
as supplied and has no `info.license` declaration; the code licence does not
relicense that document. This client is independent and is not endorsed by Ticker.
API access and use of market data remain subject to the provider's terms.

Provider documentation: [developer portal](https://developers.ticker.app/).
