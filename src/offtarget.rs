//! Remote offtarget HTTP client (`https://offtarget.0bot.dev`).
//!
//! Faithful port of the Node/Bun `offtarget-remote` helper:
//! `GET /health`, `GET /v1/db/info`, and batched `POST /v1/offtarget/check`
//! (≤50 queries per request). Types use the remote snake_case JSON names.

use crate::error::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Default origin used by the Node prototype.
pub const DEFAULT_OFFTARGET_URL: &str = "https://offtarget.0bot.dev";
/// Environment override for [`DEFAULT_OFFTARGET_URL`].
pub const OFFTARGET_URL_ENV: &str = "OFFTARGET_URL";
/// Remote check endpoint accepts at most this many queries per POST.
pub const QUERY_BATCH_SIZE: usize = 50;

/// Specificity labels returned by the t6b profile (Chinese).
pub const SPECIFICITY_HIGH: &str = "高";
pub const SPECIFICITY_MEDIUM: &str = "中";
pub const SPECIFICITY_LOW: &str = "低";

fn default_max_hits_per_query() -> u32 {
    5
}

/// `GET /health` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
}

/// One scan profile advertised by `GET /v1/db/info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbProfile {
    pub name: String,
    pub window: String,
    pub strands: String,
}

/// `GET /v1/db/info` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbInfo {
    pub transcripts: u64,
    pub bases: u64,
    pub shards: Vec<String>,
    pub fingerprint: String,
    pub indexed: bool,
    pub includes_xm_xr: bool,
    pub indexed_at: u64,
    pub profiles: Vec<DbProfile>,
}

/// One siRNA query sent to `/v1/offtarget/check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OfftargetQuery {
    pub id: String,
    pub guide: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sense: String,
}

impl OfftargetQuery {
    pub fn new(id: impl Into<String>, guide: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            guide: guide.into(),
            sense: String::new(),
        }
    }

    /// Normalize `guide` / `sense` with [`guide_to_dna`].
    pub fn normalized(&self) -> Self {
        Self {
            id: self.id.clone(),
            guide: guide_to_dna(&self.guide),
            sense: if self.sense.is_empty() {
                String::new()
            } else {
                guide_to_dna(&self.sense)
            },
        }
    }
}

/// `POST /v1/offtarget/check` request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CheckRequest {
    #[serde(default)]
    pub target_gene: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_accessions: Vec<String>,
    pub queries: Vec<OfftargetQuery>,
    #[serde(default)]
    pub scan_sense: bool,
    #[serde(default = "default_max_hits_per_query")]
    pub max_hits_per_query: u32,
}

impl CheckRequest {
    pub fn new(target_gene: impl Into<String>, queries: Vec<OfftargetQuery>) -> Self {
        Self {
            target_gene: target_gene.into(),
            target_accessions: Vec::new(),
            queries,
            scan_sense: false,
            max_hits_per_query: default_max_hits_per_query(),
        }
    }

    fn with_queries(&self, queries: Vec<OfftargetQuery>) -> Self {
        Self {
            target_gene: self.target_gene.clone(),
            target_accessions: self.target_accessions.clone(),
            queries,
            scan_sense: self.scan_sense,
            max_hits_per_query: self.max_hits_per_query,
        }
    }
}

/// On-target hit counts on a check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OnTargetCounts {
    pub transcripts: u32,
    pub genes: u32,
}

/// Off-target mismatch / seed counts (t6b profile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OfftargetCounts {
    pub perfect: u32,
    pub mismatch_1: u32,
    pub mismatch_2: u32,
    pub mismatch_3: u32,
    pub seed_transcripts: u32,
    pub seed_genes: u32,
}

/// One alignment hit under a check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OfftargetHit {
    pub accession: String,
    pub gene: String,
    pub mismatches: u32,
    pub position: u32,
    pub on_target: bool,
    pub site: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strand: Option<String>,
}

/// One per-query row from `/v1/offtarget/check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OfftargetResult {
    pub id: String,
    pub guide: String,
    pub length: u32,
    pub cached: bool,
    pub profile: String,
    #[serde(default)]
    pub specificity_label: String,
    #[serde(default)]
    pub on_target: OnTargetCounts,
    #[serde(default)]
    pub offtarget: OfftargetCounts,
    #[serde(default)]
    pub hits: Vec<OfftargetHit>,
}

/// `POST /v1/offtarget/check` response wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CheckResponse {
    pub results: Vec<OfftargetResult>,
}

/// Uppercase, U→T, strip whitespace. Does **not** drop non-ACGT bases.
pub fn guide_to_dna(seq: &str) -> String {
    seq.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u == 'U' {
                'T'
            } else {
                u
            }
        })
        .collect()
}

/// When `hide_less_specific`, drop candidates labelled `低`.
pub fn passes_specificity_filter(label: &str, hide_less_specific: bool) -> bool {
    !(hide_less_specific && label == SPECIFICITY_LOW)
}

/// Resolve the client origin: `OFFTARGET_URL` if set and non-empty, else default.
pub fn default_base_url() -> String {
    base_url_from_env(std::env::var(OFFTARGET_URL_ENV).ok().as_deref())
}

fn base_url_from_env(val: Option<&str>) -> String {
    val.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_OFFTARGET_URL)
        .to_string()
}

/// reqwest client pointed at the offtarget service (or a test double via [`Self::with_base`]).
#[derive(Debug, Clone)]
pub struct OfftargetClient {
    http: reqwest::Client,
    base: String,
}

impl OfftargetClient {
    pub fn new() -> Result<Self, Error> {
        Self::with_base(default_base_url())
    }

    pub fn with_base(base: impl Into<String>) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("sirna-design/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| Error::Http(e.to_string()))?;
        Ok(Self {
            http,
            base: base.into(),
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub async fn health(&self) -> Result<HealthStatus, Error> {
        self.get_json("/health").await
    }

    pub async fn db_info(&self) -> Result<DbInfo, Error> {
        self.get_json("/v1/db/info").await
    }

    /// POST `/v1/offtarget/check`, chunking `queries` into batches of [`QUERY_BATCH_SIZE`].
    pub async fn check(&self, req: &CheckRequest) -> Result<CheckResponse, Error> {
        if req.queries.is_empty() {
            return Ok(CheckResponse {
                results: Vec::new(),
            });
        }
        let queries: Vec<OfftargetQuery> =
            req.queries.iter().map(OfftargetQuery::normalized).collect();
        let mut results = Vec::with_capacity(queries.len());
        for chunk in queries.chunks(QUERY_BATCH_SIZE) {
            let batch = req.with_queries(chunk.to_vec());
            let mut page: CheckResponse = self.post_json("/v1/offtarget/check", &batch).await?;
            results.append(&mut page.results);
        }
        Ok(CheckResponse { results })
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.send_json(path, None::<&()>).await
    }

    async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.send_json(path, Some(body)).await
    }

    async fn send_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, Error> {
        const ATTEMPTS: u32 = 4;
        let mut last = None;
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                let backoff = Duration::from_millis(300 * (1 << (attempt - 1)));
                tracing::debug!(path, attempt, ?backoff, "offtarget retry");
                tokio::time::sleep(backoff).await;
            }
            match self.send_json_once(path, body).await {
                Ok(v) => return Ok(v),
                Err(e) if retryable(&e) && attempt + 1 < ATTEMPTS => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or_else(|| Error::Http(format!("{path}: retry exhausted"))))
    }

    async fn send_json_once<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        tracing::debug!(%url, post = body.is_some(), "offtarget HTTP");
        let builder = if body.is_some() {
            self.http.post(&url)
        } else {
            self.http.get(&url)
        };
        let builder = builder
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        let builder = if let Some(body) = body {
            builder.json(body)
        } else {
            builder
        };
        let resp = builder
            .send()
            .await
            .map_err(|e| Error::Http(format!("{path}: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Http(format!("{path}: {e}")))?;
        if !status.is_success() {
            let message = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
                .filter(|s| !s.is_empty())
                .unwrap_or(text);
            return Err(Error::OfftargetHttp {
                status: status.as_u16(),
                message,
            });
        }
        serde_json::from_str(&text).map_err(|e| Error::Msg(format!("invalid offtarget JSON: {e}")))
    }
}

/// `GET /health` against the default (or `OFFTARGET_URL`) origin.
pub async fn health() -> Result<HealthStatus, Error> {
    OfftargetClient::new()?.health().await
}

/// `GET /v1/db/info` against the default origin.
pub async fn db_info() -> Result<DbInfo, Error> {
    OfftargetClient::new()?.db_info().await
}

/// Batched `POST /v1/offtarget/check` against the default origin.
pub async fn check(req: &CheckRequest) -> Result<CheckResponse, Error> {
    OfftargetClient::new()?.check(req).await
}

fn retryable(err: &Error) -> bool {
    match err {
        Error::Http(msg) => {
            let m = msg.to_ascii_lowercase();
            m.contains("timed out")
                || m.contains("timeout")
                || m.contains("connection")
                || m.contains("decode")
        }
        Error::OfftargetHttp { status, .. } => matches!(*status, 429 | 500 | 502 | 503 | 504),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guide_to_dna_upper_u_to_t_strips_ws() {
        assert_eq!(guide_to_dna("ucau uga\nUGA"), "TCATTGATGAT");
        assert_eq!(guide_to_dna("  tcattgat  "), "TCATTGAT");
        assert_eq!(
            guide_to_dna("TCATTGATGACATCTTTGGCA"),
            "TCATTGATGACATCTTTGGCA"
        );
        assert_eq!(guide_to_dna(""), "");
        // non-ACGT kept (unlike design::normalize_dna)
        assert_eq!(guide_to_dna("acgtN-u"), "ACGTN-T");
    }

    #[test]
    fn passes_specificity_filter_drops_low_only_when_hiding() {
        assert!(passes_specificity_filter(SPECIFICITY_HIGH, true));
        assert!(passes_specificity_filter(SPECIFICITY_MEDIUM, true));
        assert!(!passes_specificity_filter(SPECIFICITY_LOW, true));
        assert!(passes_specificity_filter(SPECIFICITY_LOW, false));
        assert!(passes_specificity_filter("", true));
        assert!(passes_specificity_filter("高", false));
    }

    #[test]
    fn base_url_from_env_overrides_or_falls_back() {
        assert_eq!(base_url_from_env(None), DEFAULT_OFFTARGET_URL);
        assert_eq!(base_url_from_env(Some("")), DEFAULT_OFFTARGET_URL);
        assert_eq!(base_url_from_env(Some("   ")), DEFAULT_OFFTARGET_URL);
        assert_eq!(
            base_url_from_env(Some(" http://127.0.0.1:9 ")),
            "http://127.0.0.1:9"
        );
    }

    #[test]
    fn check_request_serializes_snake_case() {
        let req = CheckRequest {
            target_gene: "PCSK9".into(),
            target_accessions: vec!["NM_001407241.1".into()],
            queries: vec![OfftargetQuery {
                id: "si-01".into(),
                guide: "ucauugaugacaucuuuggca".into(),
                sense: "ccaaagaugucaucaaugagg".into(),
            }],
            scan_sense: false,
            max_hits_per_query: 5,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v,
            json!({
                "target_gene": "PCSK9",
                "target_accessions": ["NM_001407241.1"],
                "queries": [{
                    "id": "si-01",
                    "guide": "ucauugaugacaucuuuggca",
                    "sense": "ccaaagaugucaucaaugagg"
                }],
                "scan_sense": false,
                "max_hits_per_query": 5
            })
        );
    }

    #[test]
    fn query_normalized_applies_guide_to_dna() {
        let q = OfftargetQuery {
            id: "si-01".into(),
            guide: " uca uuga ".into(),
            sense: "cca aag".into(),
        }
        .normalized();
        assert_eq!(q.guide, "TCATTGA");
        assert_eq!(q.sense, "CCAAAG");
    }

    #[test]
    fn retryable_timeouts_and_transient_http() {
        assert!(retryable(&Error::Http(
            "error sending request for url (timed out)".into()
        )));
        assert!(retryable(&Error::OfftargetHttp {
            status: 503,
            message: "unavailable".into()
        }));
        assert!(!retryable(&Error::OfftargetHttp {
            status: 400,
            message: "queries must not be empty".into()
        }));
        assert!(!retryable(&Error::Msg("invalid offtarget JSON".into())));
    }
}
