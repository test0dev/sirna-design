//! Public REST API: axum oneshot tests (no live Ensembl / offtarget).

use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{json, Value};
use sirna_design::{
    router, AppState, Cds, CheckRequest, DesignInput, DesignResult, OfftargetQuery,
    DEFAULT_OFFTARGET_URL, ENSEMBL_REST,
};
use std::fs;
use std::path::PathBuf;
use tower::ServiceExt;

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/pcsk9")
        .join(name)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaFile {
    input: DesignInput,
    cds: Cds,
    transcript: sirna_design::Transcript,
}

fn load_meta() -> MetaFile {
    let raw = fs::read_to_string(testdata("pcsk9.meta.json")).expect("meta");
    serde_json::from_str(&raw).expect("parse meta")
}

fn load_design_golden() -> DesignResult {
    let raw = fs::read_to_string(testdata("pcsk9.design.default.json")).expect("design");
    serde_json::from_str(&raw).expect("parse design")
}

fn dummy_state() -> AppState {
    AppState::with_origins("http://127.0.0.1:1", "http://127.0.0.1:1").expect("state")
}

async fn json_body(res: axum::http::Response<axum::body::Body>) -> Value {
    let bytes = res.into_body().collect().await.expect("body").to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

async fn send(
    app: axum::Router,
    req: axum::http::Request<axum::body::Body>,
) -> axum::http::Response<axum::body::Body> {
    app.oneshot(req).await.expect("oneshot")
}

fn get(uri: &str) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .uri(uri)
        .body(axum::body::Body::empty())
        .expect("get")
}

fn post_json(uri: &str, body: &Value) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(body).expect("ser"),
        ))
        .expect("post")
}

#[tokio::test]
async fn health_ok() {
    let app = router(dummy_state());
    let res = send(app, get("/health")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn cors_allows_local_next_origin() {
    let app = router(dummy_state());
    let req = axum::http::Request::builder()
        .uri("/health")
        .header(axum::http::header::ORIGIN, "http://localhost:3000")
        .body(axum::body::Body::empty())
        .expect("cors get");
    let res = send(app, req).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let allow = res
        .headers()
        .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok());
    assert_eq!(allow, Some("http://localhost:3000"));
}

#[tokio::test]
async fn cors_preflight_design() {
    let app = router(dummy_state());
    let req = axum::http::Request::builder()
        .method("OPTIONS")
        .uri("/v1/design")
        .header(axum::http::header::ORIGIN, "http://127.0.0.1:3000")
        .header(
            axum::http::header::ACCESS_CONTROL_REQUEST_METHOD,
            "POST",
        )
        .header(
            axum::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
            "content-type",
        )
        .body(axum::body::Body::empty())
        .expect("preflight");
    let res = send(app, req).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let allow = res
        .headers()
        .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok());
    assert_eq!(allow, Some("http://127.0.0.1:3000"));
}

#[tokio::test]
async fn version_matches_crate() {
    let app = router(dummy_state());
    let res = send(app, get("/v1/version")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(body["name"], env!("CARGO_PKG_NAME"));
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn design_from_pcsk9_sequence_matches_golden() {
    let meta = load_meta();
    let mut body = serde_json::to_value(&meta.input).expect("input json");
    {
        let obj = body.as_object_mut().expect("obj");
        obj.insert("cds".into(), serde_json::to_value(meta.cds).expect("cds"));
        obj.insert("name".into(), json!(meta.transcript.name));
    }

    let app = router(dummy_state());
    let res = send(app, post_json("/v1/design", &body)).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK, "design status");
    let got: DesignResult = serde_json::from_value(json_body(res).await).expect("design result");
    let exp = load_design_golden();

    assert_eq!(got.cds, exp.cds);
    assert_eq!(got.design, exp.design);
    assert_eq!(got.transcript.id, exp.transcript.id);
    assert_eq!(got.transcript.symbol, exp.transcript.symbol);
    assert_eq!(got.transcript.name, exp.transcript.name);
    assert_eq!(got.transcript.length, exp.transcript.length);
    assert_eq!(got.sirnas.len(), exp.sirnas.len());
    pretty_assertions::assert_eq!(got.sirnas, exp.sirnas);
}

#[tokio::test]
async fn design_requires_symbol_or_sequence() {
    let app = router(dummy_state());
    let res = send(app, post_json("/v1/design", &json!({}))).await;
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = json_body(res).await;
    assert!(
        body["error"].as_str().unwrap_or("").contains("symbol"),
        "{body}"
    );
}

#[tokio::test]
async fn offtarget_check_empty_queries_short_circuits() {
    let app = router(dummy_state());
    let req = CheckRequest::new("PCSK9", Vec::new());
    let res = send(
        app,
        post_json("/v1/offtarget/check", &serde_json::to_value(&req).unwrap()),
    )
    .await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(body["results"], json!([]));
}

#[tokio::test]
async fn offtarget_check_proxies_mocked_upstream() {
    let golden_raw = fs::read_to_string(testdata("pcsk9.offtarget-remote.json")).expect("ot");
    let golden: Value = serde_json::from_str(&golden_raw).expect("ot json");
    let results = golden["results"].clone();

    let offtarget_base = spawn_offtarget_mock(results.clone()).await;
    let state = AppState::with_origins("http://127.0.0.1:1", offtarget_base).expect("state");
    let app = router(state);

    let req = CheckRequest::new(
        "PCSK9",
        vec![OfftargetQuery {
            id: "si-01".into(),
            guide: "TCATTGATGACATCTTTGGCA".into(),
            sense: "CCAAAGATGTCATCAATGAGG".into(),
        }],
    );
    let res = send(
        app,
        post_json("/v1/offtarget/check", &serde_json::to_value(&req).unwrap()),
    )
    .await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    pretty_assertions::assert_eq!(body["results"], results);
}

async fn spawn_offtarget_mock(results: Value) -> String {
    use axum::extract::State;
    use axum::routing::post;
    use axum::Json;
    use axum::Router;
    use tokio::net::TcpListener;

    #[derive(Clone)]
    struct OtState {
        results: Value,
    }

    async fn handler(State(st): State<OtState>) -> Json<Value> {
        Json(json!({ "results": st.results }))
    }

    let app = Router::new()
        .route("/v1/offtarget/check", post(handler))
        .with_state(OtState { results });
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://{addr}")
}

#[tokio::test]
#[ignore = "live Ensembl REST; run with cargo test -- --ignored"]
async fn live_design_pcsk9_symbol() {
    assert_eq!(ENSEMBL_REST, "https://rest.ensembl.org");
    let state = AppState::from_env().expect("state");
    let app = router(state);
    let res = send(
        app,
        post_json(
            "/v1/design",
            &json!({
                "symbol": "PCSK9",
                "specificity": "none",
                "hideLessSpecific": false,
                "showOffTargetHits": false
            }),
        ),
    )
    .await;
    assert_eq!(res.status(), axum::http::StatusCode::OK, "live design");
    let got: DesignResult = serde_json::from_value(json_body(res).await).expect("result");
    assert_eq!(got.transcript.symbol.to_ascii_uppercase(), "PCSK9");
    assert!(!got.sirnas.is_empty());
}

#[tokio::test]
#[ignore = "live offtarget.0bot.dev; run with cargo test -- --ignored"]
async fn live_offtarget_proxy_health_shape() {
    assert_eq!(DEFAULT_OFFTARGET_URL, "https://offtarget.0bot.dev");
    let state = AppState::from_env().expect("state");
    let app = router(state);
    let req = CheckRequest::new(
        "PCSK9",
        vec![OfftargetQuery::new("si-01", "TCATTGATGACATCTTTGGCA")],
    );
    let res = send(
        app,
        post_json("/v1/offtarget/check", &serde_json::to_value(&req).unwrap()),
    )
    .await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(body["results"][0]["id"], "si-01");
    assert!(!body["results"][0]["specificity_label"]
        .as_str()
        .unwrap_or("")
        .is_empty());
}
