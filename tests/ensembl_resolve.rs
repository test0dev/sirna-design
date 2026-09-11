//! Ensembl resolve: mocked HTTP (axum) + optional live PCSK9 smoke.

use serde_json::{json, Value};
use sirna_design::{
    default_input, design_from_resolved, locate_cds, Cds, EnsemblClient, ENSEMBL_REST,
};
use std::fs;
use std::path::PathBuf;

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/pcsk9")
        .join(name)
}

fn mock_routes(path: &str, query: Option<&str>) -> Option<(u16, Value)> {
    match (path, query.unwrap_or("")) {
        ("/lookup/symbol/homo_sapiens/PCSK9", _) => Some((
            200,
            json!({
                "id": "ENSG00000169174",
                "display_name": "PCSK9",
                "description": "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]"
            }),
        )),
        ("/lookup/symbol/homo_sapiens/NOTAGENE", _) => Some((
            400,
            json!({ "error": "No valid lookup found for symbol NOTAGENE" }),
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
                {
                    "dbname": null,
                    "display_id": null,
                    "synonyms": [null, "x"]
                },
                {
                    "dbname": "RefSeq_mRNA",
                    "display_id": "NM_001407241.1",
                    "info_text": "Generated via otherfeatures"
                },
                {
                    "dbname": "RefSeq_mRNA",
                    "display_id": "NM_174936.4",
                    "primary_id": null,
                    "info_text": "MANE Select",
                    "mystery": { "ok": true }
                }
            ]),
        )),
        ("/sequence/id/ENST00000302118", "type=cdna") => Some((
            200,
            json!({ "id": "ENST00000302118", "seq": "AAAATGCCCTAA" }),
        )),
        ("/sequence/id/ENST00000302118", "type=cds") => {
            Some((200, json!({ "id": "ENST00000302118", "seq": "ATGCCC" })))
        }
        _ => None,
    }
}

async fn spawn_mock() -> String {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum::Router;
    use tokio::net::TcpListener;

    async fn fallback(req: Request<Body>) -> Response {
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|s| s.to_string());
        match mock_routes(&path, query.as_deref()) {
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

#[tokio::test]
async fn reqwest_client_hits_mocked_rest_paths() {
    let base = spawn_mock().await;
    let client = EnsemblClient::with_base(&base).expect("client");
    let got = client.resolve_by_symbol("pcsk9").await.expect("resolve");
    assert_eq!(got.symbol, "PCSK9");
    assert_eq!(got.name, "proprotein convertase subtilisin/kexin type 9");
    assert_eq!(got.accession, "NM_174936.4");
    assert_eq!(got.ensembl_transcript, "ENST00000302118");
    assert_eq!(got.cdna, "AAAATGCCCTAA");
    assert_eq!(got.cds, Cds { start: 4, end: 9 });

    let err = client
        .resolve_by_symbol("notagene")
        .await
        .expect_err("unknown symbol");
    let msg = err.to_string();
    assert!(msg.contains("400"), "{msg}");
    assert!(msg.contains("NOTAGENE"), "{msg}");
}

#[test]
fn golden_pcsk9_cdna_cds_locate_and_design() {
    #[derive(serde::Deserialize)]
    struct Meta {
        input: sirna_design::DesignInput,
        cds: Cds,
        transcript: sirna_design::Transcript,
    }
    let raw = fs::read_to_string(testdata("pcsk9.meta.json")).expect("meta");
    let meta: Meta = serde_json::from_str(&raw).expect("parse");
    let cds_seq: String = meta
        .transcript
        .sequence
        .chars()
        .skip(meta.cds.start as usize - 1)
        .take((meta.cds.end - meta.cds.start + 1) as usize)
        .collect();
    assert_eq!(locate_cds(&meta.transcript.sequence, &cds_seq), meta.cds);

    let designed = design_from_resolved(&meta.input, &meta.transcript, meta.cds).expect("design");
    assert_eq!(designed.cds, meta.cds);
    assert!(!designed.sirnas.is_empty());
}

#[tokio::test]
#[ignore = "live Ensembl REST; run with cargo test -- --ignored"]
async fn live_resolve_pcsk9() {
    assert_eq!(ENSEMBL_REST, "https://rest.ensembl.org");
    let client = EnsemblClient::new().expect("client");
    let got = client
        .resolve_by_symbol("pcsk9")
        .await
        .expect("live resolve PCSK9");
    assert_eq!(got.symbol.to_ascii_uppercase(), "PCSK9");
    assert_eq!(got.name, "proprotein convertase subtilisin/kexin type 9");
    assert!(
        got.accession.starts_with("NM_"),
        "expected RefSeq accession, got {}",
        got.accession
    );
    assert!(
        got.ensembl_transcript.starts_with("ENST"),
        "expected ENST, got {}",
        got.ensembl_transcript
    );
    assert!(got.cds.start >= 1);
    assert!(got.cds.end >= got.cds.start);
    assert!((got.cds.end as usize) <= got.cdna.len());
    assert!(!got.name.contains("[Source:"));

    let mut input = default_input();
    input.specificity = "none".into();
    let designed = design_from_resolved(&input, &got.to_transcript(), got.cds).expect("design");
    assert_eq!(designed.transcript.symbol.to_ascii_uppercase(), "PCSK9");
    assert!(!designed.sirnas.is_empty());
}
