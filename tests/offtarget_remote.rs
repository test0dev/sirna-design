//! Remote offtarget client: golden serde + mocked HTTP (axum) + optional live health.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sirna_design::{
    guide_to_dna, passes_specificity_filter, CheckRequest, CheckResponse, OfftargetClient,
    OfftargetQuery, OfftargetResult, DEFAULT_OFFTARGET_URL, QUERY_BATCH_SIZE, SPECIFICITY_HIGH,
    SPECIFICITY_LOW, SPECIFICITY_MEDIUM,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/pcsk9")
        .join(name)
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct GoldenFile {
    base: String,
    request: CheckRequest,
    results: Vec<OfftargetResult>,
}

fn load_golden() -> GoldenFile {
    let raw = fs::read_to_string(testdata("pcsk9.offtarget-remote.json")).expect("read golden");
    serde_json::from_str(&raw).expect("parse golden")
}

#[test]
fn golden_request_and_results_round_trip() {
    let golden = load_golden();
    assert_eq!(golden.base, DEFAULT_OFFTARGET_URL);
    assert_eq!(golden.request.target_gene, "PCSK9");
    assert_eq!(golden.request.queries.len(), 1);
    assert_eq!(golden.request.queries[0].id, "si-01");
    assert_eq!(golden.request.queries[0].guide, "TCATTGATGACATCTTTGGCA");
    assert_eq!(golden.results.len(), 1);
    assert_eq!(golden.results[0].specificity_label, SPECIFICITY_MEDIUM);
    assert_eq!(golden.results[0].profile, "t6b");
    assert_eq!(golden.results[0].hits.len(), 5);

    let req_json = serde_json::to_value(&golden.request).expect("ser request");
    let req2: CheckRequest = serde_json::from_value(req_json).expect("de request");
    assert_eq!(req2, golden.request);

    let results_json = serde_json::to_value(&golden.results).expect("ser results");
    let results2: Vec<OfftargetResult> = serde_json::from_value(results_json).expect("de results");
    pretty_assertions::assert_eq!(results2, golden.results);

    let wrapped = CheckResponse {
        results: golden.results.clone(),
    };
    let wrap_json = serde_json::to_value(&wrapped).expect("ser wrap");
    let wrap2: CheckResponse = serde_json::from_value(wrap_json).expect("de wrap");
    pretty_assertions::assert_eq!(wrap2.results, golden.results);
}

#[test]
fn guide_to_dna_and_specificity_filter_exported() {
    assert_eq!(guide_to_dna(" ucaUUga\tt "), "TCATTGAT");
    assert!(passes_specificity_filter(SPECIFICITY_HIGH, true));
    assert!(passes_specificity_filter(SPECIFICITY_MEDIUM, true));
    assert!(!passes_specificity_filter(SPECIFICITY_LOW, true));
    assert!(passes_specificity_filter(SPECIFICITY_LOW, false));
}

struct MockState {
    checks: Mutex<Vec<Value>>,
    golden_results: Vec<OfftargetResult>,
}

async fn spawn_mock(golden_results: Vec<OfftargetResult>) -> (String, Arc<MockState>) {
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use axum::Json;
    use axum::Router;
    use tokio::net::TcpListener;

    let state = Arc::new(MockState {
        checks: Mutex::new(Vec::new()),
        golden_results,
    });

    async fn health() -> impl IntoResponse {
        Json(json!({ "status": "ok" }))
    }

    async fn db_info() -> impl IntoResponse {
        Json(json!({
            "transcripts": 273608,
            "bases": 1090299453,
            "shards": ["human.1.rna.fna"],
            "fingerprint": "test-fp",
            "indexed": true,
            "includes_xm_xr": true,
            "indexed_at": 1789011508,
            "profiles": [{
                "name": "t6b",
                "window": "full_guide",
                "strands": "guide (optional sense via scan_sense)"
            }]
        }))
    }

    async fn check(
        State(state): State<Arc<MockState>>,
        Json(body): Json<Value>,
    ) -> impl IntoResponse {
        let queries = body
            .get("queries")
            .and_then(|q| q.as_array())
            .cloned()
            .unwrap_or_default();
        if queries.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "queries must not be empty" })),
            )
                .into_response();
        }
        if queries.len() > QUERY_BATCH_SIZE {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "too many queries" })),
            )
                .into_response();
        }
        state.checks.lock().expect("lock").push(body.clone());

        // Echo one result per query: first query gets the golden row (id rewritten),
        // extras get a stub so batching tests can count rows.
        let mut results = Vec::new();
        for (i, q) in queries.iter().enumerate() {
            let id = q
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let guide = q
                .get("guide")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if i == 0 {
                if let Some(mut row) = state.golden_results.first().cloned() {
                    row.id = id;
                    if !guide.is_empty() {
                        row.guide = guide;
                    }
                    results.push(serde_json::to_value(row).expect("row"));
                    continue;
                }
            }
            results.push(json!({
                "id": id,
                "guide": guide,
                "length": guide.len(),
                "cached": false,
                "profile": "t6b",
                "specificity_label": "中",
                "on_target": { "transcripts": 0, "genes": 0 },
                "offtarget": {
                    "perfect": 0,
                    "mismatch_1": 0,
                    "mismatch_2": 0,
                    "mismatch_3": 0,
                    "seed_transcripts": 0,
                    "seed_genes": 0
                },
                "hits": []
            }));
        }
        Json(json!({ "results": results })).into_response()
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/db/info", get(db_info))
        .route("/v1/offtarget/check", post(check))
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (format!("http://{addr}"), state)
}

#[tokio::test]
async fn mock_check_parses_golden_results() {
    let golden = load_golden();
    let (base, state) = spawn_mock(golden.results.clone()).await;
    let client = OfftargetClient::with_base(&base).expect("client");

    let health = client.health().await.expect("health");
    assert_eq!(health.status, "ok");

    let info = client.db_info().await.expect("db info");
    assert!(info.indexed);
    assert_eq!(info.transcripts, 273608);
    assert_eq!(info.profiles[0].name, "t6b");

    let got = client.check(&golden.request).await.expect("check");
    pretty_assertions::assert_eq!(got.results, golden.results);

    let posted = state.checks.lock().expect("lock");
    assert_eq!(posted.len(), 1);
    let body = &posted[0];
    assert_eq!(body["target_gene"], "PCSK9");
    assert_eq!(body["scan_sense"], false);
    assert_eq!(body["max_hits_per_query"], 5);
    assert_eq!(body["queries"][0]["id"], "si-01");
    assert_eq!(body["queries"][0]["guide"], "TCATTGATGACATCTTTGGCA");
    assert_eq!(body["queries"][0]["sense"], "CCAAAGATGTCATCAATGAGG");
}

#[tokio::test]
async fn mock_check_normalizes_guides_and_chunks_batches() {
    let golden = load_golden();
    let (base, state) = spawn_mock(golden.results.clone()).await;
    let client = OfftargetClient::with_base(&base).expect("client");

    let mut queries = Vec::new();
    for i in 0..(QUERY_BATCH_SIZE + 1) {
        queries.push(OfftargetQuery {
            id: format!("si-{i:02}"),
            guide: " ucauugaugacaucuuuggca ".into(),
            sense: String::new(),
        });
    }
    let req = CheckRequest::new("PCSK9", queries);
    let got = client.check(&req).await.expect("check");
    assert_eq!(got.results.len(), QUERY_BATCH_SIZE + 1);
    assert_eq!(got.results[0].id, "si-00");
    assert_eq!(
        got.results[QUERY_BATCH_SIZE].id,
        format!("si-{QUERY_BATCH_SIZE:02}")
    );

    let posted = state.checks.lock().expect("lock");
    assert_eq!(posted.len(), 2);
    assert_eq!(
        posted[0]["queries"].as_array().unwrap().len(),
        QUERY_BATCH_SIZE
    );
    assert_eq!(posted[1]["queries"].as_array().unwrap().len(), 1);
    assert_eq!(posted[0]["queries"][0]["guide"], "TCATTGATGACATCTTTGGCA");
    assert_eq!(posted[0]["target_gene"], "PCSK9");
    assert_eq!(posted[1]["target_gene"], "PCSK9");
}

#[tokio::test]
async fn mock_check_empty_queries_skips_http() {
    let (base, state) = spawn_mock(Vec::new()).await;
    let client = OfftargetClient::with_base(&base).expect("client");
    let got = client
        .check(&CheckRequest::new("PCSK9", Vec::new()))
        .await
        .expect("empty");
    assert!(got.results.is_empty());
    assert!(state.checks.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn mock_check_surfaces_http_error() {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::post;
    use axum::Json;
    use axum::Router;
    use tokio::net::TcpListener;

    async fn boom() -> impl IntoResponse {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "queries must not be empty" })),
        )
    }
    let app = Router::new().route("/v1/offtarget/check", post(boom));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    let client = OfftargetClient::with_base(format!("http://{addr}")).expect("client");
    let err = client
        .check(&CheckRequest::new(
            "PCSK9",
            vec![OfftargetQuery::new("si-01", "ACGT")],
        ))
        .await
        .expect_err("400");
    let msg = err.to_string();
    assert!(msg.contains("400"), "{msg}");
    assert!(msg.contains("queries must not be empty"), "{msg}");
}

#[tokio::test]
#[ignore = "live offtarget.0bot.dev; run with cargo test -- --ignored"]
async fn live_health_offtarget() {
    assert_eq!(DEFAULT_OFFTARGET_URL, "https://offtarget.0bot.dev");
    let client = OfftargetClient::with_base(DEFAULT_OFFTARGET_URL).expect("client");
    let health = client.health().await.expect("live health");
    assert_eq!(health.status, "ok");
}
