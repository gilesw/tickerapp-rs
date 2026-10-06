use serde_json::{Value, json};
use tickerapp::{
    CategoryKind, Client, DisclosureQuery, Error, GetCurrentPriceChangeBasis, GetTimeseriesKey,
    PageQuery, Paging, TimeseriesQuery, timeseries_items,
};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn client(server: &MockServer) -> Client {
    Client::new("test-secret").with_base_url(format!("{}/v2/", server.uri()))
}
fn row() -> Value {
    json!({"date":"2025-01-28", "open":1.25, "high":1.75, "low":1.1, "close":1.5, "volume":42})
}
fn history(next: Option<&str>) -> Value {
    json!({"data":[row()], "meta":{"paging":{"pageSize":1,"nextCursor":next}}})
}
fn disclosure() -> Value {
    json!({"rnsId":"1234A", "guid":"example-1", "headline":"Example results", "timestamp":"2025-01-28T07:00:00Z", "source":"RNS", "version":null, "issuer":{"name":"Example plc"}, "category":[], "publication":[]})
}
async fn respond(server: &MockServer, route: &str, body: Value) {
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}
fn no_cursor(request: &Request) -> bool {
    !request
        .url
        .query_pairs()
        .any(|(name, _)| name == "pageCursor")
}

#[tokio::test]
async fn price_keeps_identifier_colon_and_uses_header_auth() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/prices/XLON:LLOY"))
        .and(header("x-api-key", "test-secret"))
        .and(query_param("changeBasis", "trade"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"data":{"quote":{"trade":55.42,"timestamp":"2025-01-28T16:30:00Z"}}}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let snapshot = client(&server)
        .price_with_change_basis("XLON:LLOY", Some(GetCurrentPriceChangeBasis::Trade))
        .await
        .unwrap();
    let quote = snapshot.quote.unwrap();
    assert_eq!(quote.trade, Some(55.42));
    assert_eq!(quote.mid, None);
    assert!(quote.timestamp.is_some());
    assert!(
        !server.received_requests().await.unwrap()[0]
            .url
            .as_str()
            .contains("test-secret")
    );
}

#[tokio::test]
async fn snapshot_year_dates_are_timestamps() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v2/prices/GB0008706128",
        json!({"data":{"year":{
            "high":{"price":117.9,"date":"2026-08-04T00:00:00.000000Z"},
            "low":{"price":81.82,"date":"2025-10-17T00:00:00.000000Z"},
            "volume":{"total":38872653682_i64,"average":105609996.71541502}}}}),
    )
    .await;
    let year = client(&server)
        .price("GB0008706128")
        .await
        .unwrap()
        .year
        .unwrap();
    assert_eq!(
        year.high.unwrap().date.unwrap().to_rfc3339(),
        "2026-08-04T00:00:00+00:00"
    );
    assert_eq!(year.low.unwrap().price, Some(81.82));
    assert_eq!(year.volume.unwrap().average, Some(105609996.71541502));
}

#[tokio::test]
async fn empty_and_partial_snapshots_preserve_missing_values() {
    for body in [
        json!({"data":{}}),
        json!({"data":{"session":{"close":null},"fundamentals":{"marketCap":null}}}),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/v2/prices/test", body).await;
        let snapshot = client(&server).price("test").await.unwrap();
        assert!(snapshot.quote.is_none());
        assert!(snapshot.session.and_then(|session| session.close).is_none());
    }
}

#[tokio::test]
async fn history_sends_filters_and_preserves_prices() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/prices/test/timeseries"))
        .and(query_param("dateFrom", "2025-01"))
        .and(query_param("dateTo", "2025-02"))
        .and(query_param("includeNav", "true"))
        .and(query_param("_key", "array"))
        .and(query_param("pageCursor", "next +/?"))
        .respond_with(ResponseTemplate::new(200).set_body_json(history(None)))
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server)
        .timeseries(
            "test",
            &TimeseriesQuery {
                date_from: Some("2025-01".into()),
                date_to: Some("2025-02".into()),
                include_nav: Some(true),
                key: Some(GetTimeseriesKey::Array),
                page: PageQuery {
                    page_cursor: Some("next +/?".into()),
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(timeseries_items(page.data).unwrap()[0].close, 1.5);
}

#[test]
fn keyed_history_converts_dates_and_rejects_invalid_keys() {
    for key in ["2025-01-28", "1738022400", "invalid-date"] {
        let mut value = row();
        value.as_object_mut().unwrap().remove("date");
        let data = serde_json::from_value(json!({key:value})).unwrap();
        let result = timeseries_items(data);
        if key == "invalid-date" {
            assert!(matches!(result, Err(Error::Decode(_))));
        } else {
            let rows = result.unwrap();
            assert_eq!(rows[0].date.to_string(), "2025-01-28");
            assert_eq!(rows[0].volume, 42);
        }
    }
}

#[tokio::test]
async fn incomplete_history_rows_are_errors_not_zero() {
    let server = MockServer::start().await;
    let mut body = history(None);
    body["data"][0].as_object_mut().unwrap().remove("close");
    respond(&server, "/v2/prices/test/timeseries", body).await;
    assert!(matches!(
        client(&server)
            .timeseries("test", &TimeseriesQuery::default())
            .await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn history_follows_cursor_to_completion() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/prices/test/timeseries"))
        .and(no_cursor)
        .respond_with(ResponseTemplate::new(200).set_body_json(history(Some("next"))))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/v2/prices/test/timeseries"))
        .and(query_param("pageCursor", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(history(None)))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        client(&server)
            .all_timeseries("test", &TimeseriesQuery::default(), 2)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn page_limit_and_cursor_loops_never_return_partial_success() {
    for limit in [1, 5] {
        let server = MockServer::start().await;
        respond(&server, "/v2/prices/test/timeseries", history(Some("same"))).await;
        assert!(matches!(
            client(&server)
                .all_timeseries("test", &TimeseriesQuery::default(), limit)
                .await,
            Err(Error::Pagination(_))
        ));
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            if limit == 1 { 1 } else { 2 }
        );
    }
}

#[tokio::test]
async fn unexpected_paging_mode_is_an_error() {
    let server = MockServer::start().await;
    respond(&server, "/v2/prices/test/timeseries", json!({"data":[],"meta":{"paging":{"pageSize":1,"pageNumber":1,"pageTotal":2,"itemTotal":2}}})).await;
    assert!(matches!(
        client(&server)
            .all_timeseries("test", &TimeseriesQuery::default(), 5)
            .await,
        Err(Error::Pagination(_))
    ));
}

#[tokio::test]
async fn invalid_paging_is_rejected_without_network_requests() {
    let server = MockServer::start().await;
    let client = client(&server);
    for page in [
        PageQuery {
            page_size: Some(0),
            ..Default::default()
        },
        PageQuery {
            page_number: Some(0),
            ..Default::default()
        },
        PageQuery {
            page_number: Some(1),
            page_cursor: Some("x".into()),
            ..Default::default()
        },
    ] {
        assert!(matches!(
            client.exchanges(&page).await,
            Err(Error::InvalidRequest(_))
        ));
    }
    assert!(matches!(
        client
            .all_timeseries("test", &TimeseriesQuery::default(), 0)
            .await,
        Err(Error::InvalidRequest(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn disclosures_preserve_warnings_nullable_version_and_unknown_categories() {
    let server = MockServer::start().await;
    let mut item = disclosure();
    item["category"] = json!([
        {"kind":"NewKind","code":"NEW","name":"New category","importance":"NewImportance"},
        {"kind":"RNS","code":"EOD","name":"Final Announcement Released","importance":"low"}
    ]);
    Mock::given(path("/v2/disclosures/sources/rns/items"))
        .and(query_param("symbols", "LLOY,BARC")).and(query_param("fcaCategories", "FR,IR"))
        .and(query_param("hasSymbol", "true")).and(query_param("q", "results & dividends"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[item],"meta":{"paging":{"pageSize":1,"nextCursor":"next","latestCursor":"latest"}},"warnings":["Filter unavailable on this plan"]})))
        .expect(1).mount(&server).await;
    let page = client(&server)
        .disclosures(&DisclosureQuery {
            symbols: vec!["LLOY".into(), "BARC".into()],
            fca_categories: vec!["FR".into(), "IR".into()],
            has_symbol: Some(true),
            query: Some("results & dividends".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.warnings.unwrap()[0], "Filter unavailable on this plan");
    assert_eq!(page.data[0].version, None);
    assert_eq!(page.data[0].category[0].kind.as_str(), "NewKind");
    assert!(matches!(page.data[0].category[1].kind, CategoryKind::Rns));
    assert_eq!(
        page.data[0].category[0]
            .importance
            .as_ref()
            .unwrap()
            .as_str(),
        "NewImportance"
    );
    assert!(
        matches!(page.meta.paging, Paging::CursorPaging(p) if p.next_cursor.as_deref() == Some("next") && p.latest_cursor == Some(Some("latest".into())))
    );
}

#[tokio::test]
async fn individual_disclosure_and_exchange_decode() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v2/disclosures/sources/rns/items/urn:newsml:example.com:20250128:1234A:1",
        json!({"data":disclosure()}),
    )
    .await;
    respond(
        &server,
        "/v2/references/exchanges/XLON",
        json!({"data":{"mic":"XLON","segments":[],"_links":{}}}),
    )
    .await;
    assert_eq!(
        client(&server)
            .disclosure("urn:newsml:example.com:20250128:1234A:1")
            .await
            .unwrap()
            .rns_id,
        "1234A"
    );
    assert_eq!(
        client(&server)
            .exchange("XLON")
            .await
            .unwrap()
            .data
            .unwrap()
            .mic,
        "XLON"
    );
}

#[tokio::test]
async fn exchange_pages_keep_classic_paging_and_nullable_fields() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/references/exchanges")).and(query_param("pageNumber", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"mic":"XLON","name":null}],"meta":{"paging":{"pageSize":1,"pageNumber":2,"pageTotal":2,"itemTotal":2}}})))
        .expect(1).mount(&server).await;
    let page = client(&server)
        .exchanges(&PageQuery {
            page_number: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.data.unwrap()[0].name, Some(None));
    assert!(matches!(page.meta.unwrap().paging, Paging::ClassicPaging(p) if p.page_number == 2));
}

#[tokio::test]
async fn http_errors_keep_body_and_retry_after() {
    for status in [401, 403, 404, 429, 503] {
        let server = MockServer::start().await;
        Mock::given(path("/v2/prices/test"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Retry-After", "60")
                    .set_body_json(
                        json!({"error":{"message":"Example failure","hint":"Try later"}}),
                    ),
            )
            .mount(&server)
            .await;
        let error = client(&server).price("test").await.unwrap_err();
        assert_eq!(error.is_unauthorized(), status == 401 || status == 403);
        assert_eq!(error.is_rate_limited(), status == 429);
        assert!(
            matches!(error, Error::Api{status:actual,body,retry_after} if actual == status && body.contains("Try later") && retry_after.as_deref() == Some("60"))
        );
    }
}

#[tokio::test]
async fn malformed_success_and_response_limits_fail() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v2/prices/test",
        json!({"error":{"message":"Not a price response"}}),
    )
    .await;
    assert!(matches!(
        client(&server).price("test").await,
        Err(Error::Decode(_))
    ));
    assert!(matches!(
        client(&server)
            .with_max_response_body_bytes(8)
            .price("test")
            .await,
        Err(Error::Transport(_))
    ));
}

#[tokio::test]
async fn raw_client_exposes_nav_and_market_statistics() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v2/prices/test/nav",
        json!({"data":[],"meta":{"paging":{"pageSize":1,"nextCursor":null}}}),
    )
    .await;
    respond(
        &server,
        "/v2/market-stats/risers",
        json!({"data":[],"meta":{"count":0,"exchangeMic":"XLON","tradingDate":"2025-01-28"}}),
    )
    .await;
    let client = client(&server);
    assert!(
        client
            .raw()
            .get_nav(
                "test",
                None::<&str>,
                None::<&str>,
                None::<&str>,
                None,
                None::<&str>,
                None
            )
            .await
            .unwrap()
            .data
            .is_empty()
    );
    assert_eq!(
        client
            .raw()
            .get_market_stats_risers(None::<&str>, None::<&str>, None::<&str>)
            .await
            .unwrap()
            .meta
            .exchange_mic,
        "XLON"
    );
}

#[test]
fn debug_omits_credentials() {
    assert!(!format!("{:?}", Client::new("test-secret")).contains("test-secret"));
}
