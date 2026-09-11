//! Public REST API: axum oneshot tests (no live Ensembl / offtarget).

use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{json, Value};
use sirna_design::{
    router, AppState, Cds, CheckRequest, DesignInput, DesignResult, EnsemblClient, NcbiClient,
    OfftargetQuery, ResolveCache, ResolveResponse, RetrieveResponse, DEFAULT_OFFTARGET_URL,
    ENSEMBL_REST,
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

fn state_for(
    ensembl_base: &str,
    offtarget_base: &str,
    ncbi_base: &str,
    cache: ResolveCache,
) -> AppState {
    AppState {
        ensembl: EnsemblClient::with_base(ensembl_base).expect("ensembl"),
        offtarget: sirna_design::OfftargetClient::with_base(offtarget_base).expect("ot"),
        cache,
        ncbi: NcbiClient::with_base(ncbi_base).expect("ncbi"),
    }
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
        .header(axum::http::header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
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

fn ensembl_mock_routes(path: &str, query: Option<&str>) -> Option<(u16, Value)> {
    match (path, query.unwrap_or("")) {
        ("/lookup/symbol/homo_sapiens/PCSK9", _) => Some((
            200,
            json!({
                "id": "ENSG00000169174",
                "display_name": "PCSK9",
                "description": "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]"
            }),
        )),
        ("/lookup/id/ENSG00000169174", "expand=1") => Some((
            200,
            json!({
                "id": "ENSG00000169174",
                "display_name": "PCSK9",
                "description": "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]",
                "canonical_transcript": "ENST00000302118.5",
                "Transcript": [{
                    "id": "ENST00000302118",
                    "version": 5,
                    "is_canonical": 1,
                    "biotype": "protein_coding",
                    "length": 12
                }]
            }),
        )),
        ("/xrefs/id/ENST00000302118", _) => Some((
            200,
            json!([
                null,
                { "dbname": null, "synonyms": [null] },
                {
                    "dbname": "RefSeq_mRNA",
                    "display_id": "NM_174936.4",
                    "info_text": "MANE Select",
                    "extra": true
                }
            ]),
        )),
        ("/sequence/id/ENST00000302118", "type=cdna") => Some((
            200,
            json!({
                "id": "ENST00000302118",
                "desc": "PCSK9-201",
                "seq": "AAAATGCCCTAA"
            }),
        )),
        ("/sequence/id/ENST00000302118", "type=cds") => {
            Some((200, json!({ "id": "ENST00000302118", "seq": "ATGCCC" })))
        }
        _ => None,
    }
}

async fn spawn_ensembl_mock() -> String {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum::Router;
    use tokio::net::TcpListener;

    async fn fallback(req: Request<Body>) -> Response {
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|s| s.to_string());
        match ensembl_mock_routes(&path, query.as_deref()) {
            Some((200, body)) => (StatusCode::OK, axum::Json(body)).into_response(),
            Some((code, body)) => {
                let status = StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_REQUEST);
                (status, axum::Json(body)).into_response()
            }
            None => (StatusCode::NOT_FOUND, axum::Json(json!({"error": path}))).into_response(),
        }
    }

    let app = Router::new().fallback(fallback);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://{addr}")
}

async fn spawn_ncbi_mock() -> String {
    use axum::extract::Query;
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum::Router;
    use std::collections::HashMap;
    use tokio::net::TcpListener;

    async fn efetch(Query(q): Query<HashMap<String, String>>) -> (StatusCode, String) {
        let id = q.get("id").map(|s| s.as_str()).unwrap_or("");
        if id.to_ascii_uppercase().starts_with("NM_") {
            (
                StatusCode::OK,
                format!(">{id} Homo sapiens test\nATGC\nTAAA\n"),
            )
        } else {
            (StatusCode::OK, format!("Error: Invalid uid {id}"))
        }
    }

    let app = Router::new().route("/efetch", get(efetch));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://{addr}/efetch")
}

#[tokio::test]
async fn resolve_oneshot_mock_ensembl_and_cache() {
    let ensembl = spawn_ensembl_mock().await;
    let cache = ResolveCache::in_memory().expect("cache");
    let state = state_for(
        &ensembl,
        "http://127.0.0.1:1",
        "http://127.0.0.1:1/efetch",
        cache,
    );
    let app = router(state);

    let res = send(
        app.clone(),
        get("/v1/resolve?symbol=pcsk9&include_sequence=1"),
    )
    .await;
    assert_eq!(res.status(), axum::http::StatusCode::OK, "resolve");
    let first: ResolveResponse = serde_json::from_value(json_body(res).await).expect("body");
    assert_eq!(first.symbol, "PCSK9");
    assert_eq!(first.accession, "NM_174936.4");
    assert_eq!(first.ensembl_transcript, "ENST00000302118");
    assert_eq!(first.species, "homo_sapiens");
    assert_eq!(first.sequence, "AAAATGCCCTAA");
    assert_eq!(first.length, 12);
    assert_eq!(first.cds, Cds { start: 4, end: 9 });
    assert!(!first.cached);

    let res = send(app, get("/v1/resolve?symbol=PCSK9&include_sequence=1")).await;
    let second: ResolveResponse = serde_json::from_value(json_body(res).await).expect("body");
    assert!(second.cached);
    assert_eq!(second.sequence, first.sequence);
    assert_eq!(second.accession, first.accession);
}

#[tokio::test]
async fn resolve_oneshot_omits_sequence_when_asked() {
    let ensembl = spawn_ensembl_mock().await;
    let state = state_for(
        &ensembl,
        "http://127.0.0.1:1",
        "http://127.0.0.1:1/efetch",
        ResolveCache::in_memory().expect("cache"),
    );
    let app = router(state);
    let res = send(app, get("/v1/resolve?symbol=PCSK9&include_sequence=0")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body = json_body(res).await;
    assert!(body.get("sequence").is_none(), "{body}");
    assert!(body.get("length").is_none(), "{body}");
    assert_eq!(body["ensemblTranscript"], "ENST00000302118");
    assert_eq!(body["cds"]["start"], 4);
    assert_eq!(body["cached"], false);
}

#[tokio::test]
async fn retrieve_oneshot_enst_and_ncbi() {
    let ensembl = spawn_ensembl_mock().await;
    let ncbi = spawn_ncbi_mock().await;
    let cache = ResolveCache::in_memory().expect("cache");
    let state = state_for(&ensembl, "http://127.0.0.1:1", &ncbi, cache);
    let app = router(state);

    let res = send(app.clone(), get("/v1/retrieve?accession=ENST00000302118")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let enst: RetrieveResponse = serde_json::from_value(json_body(res).await).expect("enst");
    assert_eq!(enst.accession, "ENST00000302118");
    assert_eq!(enst.sequence, "AAAATGCCCTAA");
    assert_eq!(enst.length, 12);
    assert!(!enst.cached);

    let res = send(app.clone(), get("/v1/retrieve?accession=ENST00000302118")).await;
    let cached: RetrieveResponse = serde_json::from_value(json_body(res).await).expect("cached");
    assert!(cached.cached);

    let res = send(app, get("/v1/retrieve?accession=NM_012131.3")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let nm: RetrieveResponse = serde_json::from_value(json_body(res).await).expect("nm");
    assert_eq!(nm.accession, "NM_012131.3");
    assert_eq!(nm.length, 8);
    assert!(nm.sequence.starts_with(">NM_012131.3"));
}

#[tokio::test]
async fn design_from_symbol_survives_weird_xrefs() {
    let ensembl = spawn_ensembl_mock().await;
    let state = state_for(
        &ensembl,
        "http://127.0.0.1:1",
        "http://127.0.0.1:1/efetch",
        ResolveCache::in_memory().expect("cache"),
    );
    let app = router(state);
    let res = send(
        app,
        post_json(
            "/v1/design",
            &json!({
                "symbol": "PCSK9",
                "specificity": "none",
                "hideLessSpecific": false,
                "showOffTargetHits": false,
                "gcMin": 0,
                "gcMax": 100,
                "avoidContiguousGC": false,
                "avoidContiguousAT": false,
                "seedTmMax": 100
            }),
        ),
    )
    .await;
    assert_eq!(
        res.status(),
        axum::http::StatusCode::OK,
        "design from symbol"
    );
    let body = json_body(res).await;
    assert_eq!(body["transcript"]["symbol"], "PCSK9");
    assert_eq!(body["transcript"]["id"], "NM_174936.4");
    assert_eq!(body["transcript"]["sequence"], "AAAATGCCCTAA");
}

#[tokio::test]
async fn resolve_requires_symbol() {
    let app = router(dummy_state());
    let res = send(app, get("/v1/resolve?symbol=")).await;
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "live Ensembl REST; run with cargo test -- --ignored"]
async fn live_resolve_pcsk9_api() {
    assert_eq!(ENSEMBL_REST, "https://rest.ensembl.org");
    let state = AppState::from_env().expect("state");
    let app = router(state);
    let res = send(app, get("/v1/resolve?symbol=PCSK9&include_sequence=1")).await;
    assert_eq!(res.status(), axum::http::StatusCode::OK, "live resolve");
    let got: ResolveResponse = serde_json::from_value(json_body(res).await).expect("body");
    assert_eq!(got.symbol.to_ascii_uppercase(), "PCSK9");
    assert!(got.ensembl_transcript.starts_with("ENST"));
    assert!(!got.sequence.is_empty());
    assert!(got.length as usize == got.sequence.len());
    assert!(got.cds.start >= 1);
    assert!(!got.cached);
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
