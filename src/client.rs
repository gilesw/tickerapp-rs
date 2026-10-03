use std::collections::HashSet;

use crate::{
    DisclosureItem, Error, GetCurrentPriceChangeBasis, GetExchangeResponse, GetTimeseriesKey,
    GetTimeseriesResponse, GetTimeseriesResponseData, ListExchangesResponse,
    ListRnsDisclosureItemsChannel, ListRnsDisclosureItemsResponse, Paging, PriceSnapshot,
    TimeseriesItem, generated::HttpClient,
};

pub const BASE_URL: &str = "https://api.tickerapp.net/v2";

/// One page, using either a cursor or a page number. Defaults are provider-defined.
#[derive(Debug, Clone, Default)]
pub struct PageQuery {
    pub page_size: Option<i64>,
    pub page_cursor: Option<String>,
    pub page_number: Option<i64>,
}

impl PageQuery {
    fn validate(&self) -> Result<(), Error> {
        if self.page_cursor.is_some() && self.page_number.is_some() {
            return Err(Error::InvalidRequest(
                "Choose a page cursor or a page number, not both",
            ));
        }
        if self.page_size.is_some_and(|size| size < 1)
            || self.page_number.is_some_and(|page| page < 1)
        {
            return Err(Error::InvalidRequest(
                "Page size and number must be positive",
            ));
        }
        Ok(())
    }
}

/// Daily history filters. Dates accept the provider's partial ISO8601 syntax.
#[derive(Debug, Clone, Default)]
pub struct TimeseriesQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub include_nav: Option<bool>,
    pub key: Option<GetTimeseriesKey>,
    pub page: PageQuery,
}

/// RNS filters. Responses retain provider warnings about ignored filters.
#[derive(Debug, Clone, Default)]
pub struct DisclosureQuery {
    pub symbols: Vec<String>,
    pub isins: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub fca_categories: Vec<String>,
    pub ticker_categories: Vec<String>,
    pub query: Option<String>,
    pub has_symbol: Option<bool>,
    pub channel: Option<ListRnsDisclosureItemsChannel>,
    pub page: PageQuery,
}

/// Sends credentials in the x-api-key header. No automatic retries.
#[derive(Clone)]
pub struct Client {
    inner: HttpClient,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            inner: HttpClient::new().with_api_key(api_key),
        }
    }

    /// Override the complete API root, including /v2.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .with_base_url(base_url.into().trim_end_matches('/'));
        self
    }

    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }

    /// All 14 generated operations from the pinned specification.
    pub fn raw(&self) -> &HttpClient {
        &self.inner
    }

    /// Snapshot for an ISIN or MIC:SYMBOL. Missing fields remain None.
    pub async fn price(&self, identifier: &str) -> Result<PriceSnapshot, Error> {
        self.price_with_change_basis(identifier, None).await
    }

    pub async fn price_with_change_basis(
        &self,
        identifier: &str,
        basis: Option<GetCurrentPriceChangeBasis>,
    ) -> Result<PriceSnapshot, Error> {
        Ok(self.inner.get_current_price(identifier, basis).await?.data)
    }

    /// One page; inspect meta.paging before requesting another page.
    pub async fn timeseries(
        &self,
        identifier: &str,
        query: &TimeseriesQuery,
    ) -> Result<GetTimeseriesResponse, Error> {
        query.page.validate()?;
        Ok(self
            .inner
            .get_timeseries(
                identifier,
                query.date_from.as_deref(),
                query.date_to.as_deref(),
                query.include_nav,
                query.key,
                query.page.page_size,
                query.page.page_cursor.as_deref(),
                query.page.page_number,
            )
            .await?)
    }

    /// Collect cursor pages up to an explicit request limit. Returns an error if
    /// more pages remain at the limit, a cursor repeats, or paging changes mode.
    /// No partial history is returned as if it were complete. Row order is kept.
    pub async fn all_timeseries(
        &self,
        identifier: &str,
        query: &TimeseriesQuery,
        max_pages: usize,
    ) -> Result<Vec<TimeseriesItem>, Error> {
        if max_pages == 0 || query.page.page_number.is_some() {
            return Err(Error::InvalidRequest(
                "Use cursor pagination and a positive maximum page count",
            ));
        }
        let mut query = query.clone();
        let mut seen = HashSet::new();
        if let Some(cursor) = &query.page.page_cursor {
            seen.insert(cursor.clone());
        }
        let mut rows = Vec::new();
        for _ in 0..max_pages {
            let response = self.timeseries(identifier, &query).await?;
            rows.extend(timeseries_items(response.data)?);
            match response.meta.paging {
                Paging::CursorPaging(page) => match page.next_cursor {
                    None => return Ok(rows),
                    Some(cursor) if !cursor.is_empty() && seen.insert(cursor.clone()) => {
                        query.page.page_cursor = Some(cursor);
                    }
                    Some(_) => return Err(Error::Pagination("Repeated or empty cursor")),
                },
                Paging::ClassicPaging(_) => {
                    return Err(Error::Pagination("Expected cursor pagination"));
                }
            }
        }
        Err(Error::Pagination(
            "Page limit reached while more history remains",
        ))
    }

    /// One page of disclosures, including warnings and pagination metadata.
    pub async fn disclosures(
        &self,
        query: &DisclosureQuery,
    ) -> Result<ListRnsDisclosureItemsResponse, Error> {
        query.page.validate()?;
        Ok(self
            .inner
            .list_rns_disclosure_items(
                query.date_from.as_deref(),
                query.date_to.as_deref(),
                joined(&query.isins),
                None,
                joined(&query.symbols),
                None,
                joined(&query.fca_categories),
                None,
                joined(&query.ticker_categories),
                None,
                query.query.as_deref(),
                query.page.page_size,
                query.page.page_cursor.as_deref(),
                query.page.page_number,
                query.has_symbol,
                query.channel,
            )
            .await?)
    }

    /// Fetch an individual disclosure by its provider identifier.
    pub async fn disclosure(&self, identifier: &str) -> Result<DisclosureItem, Error> {
        Ok(self.inner.get_rns_disclosure_item(identifier).await?.data)
    }

    pub async fn exchanges(&self, query: &PageQuery) -> Result<ListExchangesResponse, Error> {
        query.validate()?;
        Ok(self
            .inner
            .list_exchanges(
                query.page_size,
                query.page_cursor.as_deref(),
                query.page_number,
            )
            .await?)
    }

    pub async fn exchange(&self, mic: &str) -> Result<GetExchangeResponse, Error> {
        Ok(self.inner.get_exchange(mic).await?)
    }
}

fn joined(values: &[String]) -> Option<String> {
    (!values.is_empty()).then(|| values.join(","))
}

/// Convert array, ISO-date-keyed or Unix-seconds-keyed history into rows.
/// Invalid keys fail explicitly; rows are never silently discarded.
pub fn timeseries_items(data: GetTimeseriesResponseData) -> Result<Vec<TimeseriesItem>, Error> {
    match data {
        GetTimeseriesResponseData::TimeseriesArray(rows) => Ok(rows),
        GetTimeseriesResponseData::GetTimeseriesResponseDataVariant2(keyed) => keyed
            .additional_properties
            .into_iter()
            .map(|(key, row)| {
                let date = chrono::NaiveDate::parse_from_str(&key, "%Y-%m-%d")
                    .ok()
                    .or_else(|| {
                        key.parse::<i64>()
                            .ok()
                            .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
                            .map(|timestamp| timestamp.date_naive())
                    })
                    .ok_or_else(|| Error::Decode(format!("Invalid timeseries date key: {key}")))?;
                Ok(TimeseriesItem {
                    date,
                    open: row.open,
                    high: row.high,
                    low: row.low,
                    close: row.close,
                    volume: row.volume,
                    nav: row.nav,
                })
            })
            .collect(),
    }
}
