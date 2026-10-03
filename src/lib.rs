//! Async client for the Ticker.app v2 market data API.
//!
//! ```no_run
//! use tickerapp::{Client, DisclosureQuery};
//!
//! # async fn example() -> Result<(), tickerapp::Error> {
//! let client = Client::new("your-api-key");
//! let page = client.disclosures(&DisclosureQuery::default()).await?;
//! for item in page.data {
//!     println!("{}: {}", item.timestamp, item.headline);
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod error;

#[allow(clippy::all, dead_code, unused_imports)]
#[rustfmt::skip]
pub mod generated;

pub use client::{BASE_URL, Client, DisclosureQuery, PageQuery, TimeseriesQuery, timeseries_items};
pub use error::Error;
pub use generated::client::{
    GetCurrentPriceChangeBasis, GetTimeseriesKey, ListRnsDisclosureItemsChannel,
};
pub use generated::types::*;
