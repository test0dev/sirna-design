//! Public REST API (axum): health, version, design, offtarget proxy.
//!
//! Semantically equivalent to the Node prototype's design / check capabilities.
//! Listen address and upstream origins come from env / [`AppState`].

use crate::design::{
    design_from_resolved, design_sirnas, normalize_dna, Cds, DesignInput, DesignResult, Transcript,
};
use crate::ensembl::{EnsemblClient, DEFAULT_SPECIES, ENSEMBL_REST};
use crate::error::Error;
use crate::offtarget::{default_base_url, CheckRequest, CheckResponse, OfftargetClient};
use axum::extract::State;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Env override for [`ENSEMBL_REST`].
pub const ENSEMBL_URL_ENV: &str = "ENSEMBL_URL";
/// Env override for the bind address (also `--listen`).
pub const LISTEN_ADDR_ENV: &str = "LISTEN_ADDR";
/// Optional CORS allow-origin list (`*`, or comma-separated origins).
pub const CORS_ALLOW_ORIGIN_ENV: &str = "CORS_ALLOW_ORIGIN";
/// Default bind address.
pub const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8080";
/// Local Next.js origins allowed when [`CORS_ALLOW_ORIGIN_ENV`] is unset.
pub const DEFAULT_CORS_ORIGINS: &[&str] = &["http://localhost:3000", "http://127.0.0.1:3000"];

/// Shared upstream clients injected into the router.
#[derive(Clone)]
pub struct AppState {
    pub ensembl: EnsemblClient,
    pub offtarget: OfftargetClient,
}

impl AppState {
    pub fn from_env() -> Result<Self, Error> {
        Self::with_origins(ensembl_base_url(), default_base_url())
    }

    pub fn with_origins(
        ensembl_base: impl Into<String>,
        offtarget_base: impl Into<String>,
    ) -> Result<Self, Error> {
        Ok(Self {
            ensembl: EnsemblClient::with_base(ensembl_base)?,
            offtarget: OfftargetClient::with_base(offtarget_base)?,
        })
    }
}

/// `GET /health`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiHealth {
    pub status: String,
}

/// `GET /v1/version`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiVersion {
    pub name: String,
    pub version: String,
}

/// `POST /v1/design` body: Ensembl `symbol` **or** an explicit `sequence`.
///
/// DesignInput overrides are flattened (camelCase, same as `pcsk9.meta.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct DesignRequest {
    /// Ensembl gene symbol. Used when `sequence` is empty.
    #[serde(default)]
    pub symbol: String,
    /// Optional display name for a sequence-supplied transcript.
    #[serde(default)]
    pub name: String,
    /// Ensembl species slug (default `homo_sapiens`, or `mus_musculus` if
    /// `specificity` starts with `mouse`).
    #[serde(default)]
    pub species: String,
    /// Optional 1-based closed CDS when designing from a raw sequence.
    #[serde(default)]
    pub cds: Option<Cds>,
    #[serde(flatten)]
    pub input: DesignInput,
}

/// Build the public router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/version", get(version))
        .route("/v1/design", post(design))
        .route("/v1/offtarget/check", post(offtarget_check))
        .with_state(state)
        .layer(cors_layer())
}

/// CORS layer: `CORS_ALLOW_ORIGIN` or the local Next.js defaults.
pub fn cors_layer() -> CorsLayer {
    cors_layer_from(env_nonempty(CORS_ALLOW_ORIGIN_ENV).as_deref())
}

fn cors_layer_from(setting: Option<&str>) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(allow_origin_from(setting))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::ORIGIN])
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedCorsOrigins {
    Any,
    List(Vec<String>),
}

fn parse_cors_origins(setting: Option<&str>) -> ParsedCorsOrigins {
    match setting.map(str::trim).filter(|s| !s.is_empty()) {
        Some("*") => ParsedCorsOrigins::Any,
        Some(raw) => {
            let values: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            if values.is_empty() {
                ParsedCorsOrigins::List(default_cors_origin_list())
            } else {
                ParsedCorsOrigins::List(values)
            }
        }
        None => ParsedCorsOrigins::List(default_cors_origin_list()),
    }
}

fn allow_origin_from(setting: Option<&str>) -> AllowOrigin {
    match parse_cors_origins(setting) {
        ParsedCorsOrigins::Any => AllowOrigin::any(),
        ParsedCorsOrigins::List(origins) => {
            let values: Vec<HeaderValue> = origins
                .iter()
                .filter_map(|s| HeaderValue::from_str(s).ok())
                .collect();
            if values.is_empty() {
                AllowOrigin::list(default_cors_header_values())
            } else {
                AllowOrigin::list(values)
            }
        }
    }
}

fn default_cors_origin_list() -> Vec<String> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn default_cors_header_values() -> Vec<HeaderValue> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .map(|o| HeaderValue::from_static(o))
        .collect()
}

async fn health() -> Json<ApiHealth> {
    Json(ApiHealth {
        status: "ok".into(),
    })
}

async fn version() -> Json<ApiVersion> {
    Json(ApiVersion {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn design(
    State(state): State<AppState>,
    Json(req): Json<DesignRequest>,
) -> Result<Json<DesignResult>, ApiError> {
    Ok(Json(run_design(&state, req).await?))
}

async fn offtarget_check(
    State(state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<CheckResponse>, ApiError> {
    Ok(Json(state.offtarget.check(&req).await?))
}

async fn run_design(state: &AppState, mut req: DesignRequest) -> Result<DesignResult, Error> {
    if let Some(sym) = lookup_symbol(&req) {
        if req.input.gene_symbol.trim().is_empty() {
            req.input.gene_symbol = sym.clone();
        }
        if !has_sequence(&req) {
            return design_from_symbol(state, req, &sym).await;
        }
    }
    if has_sequence(&req) {
        return design_from_sequence(req);
    }
    Err(Error::Msg(
        "provide `symbol` (Ensembl) or `sequence` (direct design)".into(),
    ))
}

async fn design_from_symbol(
    state: &AppState,
    req: DesignRequest,
    symbol: &str,
) -> Result<DesignResult, Error> {
    let species = ensembl_species(&req.species, &req.input.specificity);
    let resolved = state
        .ensembl
        .resolve_by_symbol_for(symbol, &species)
        .await?;
    let mut input = req.input;
    if input.gene_symbol.trim().is_empty() {
        input.gene_symbol = resolved.symbol.clone();
    }
    if input.accession.trim().is_empty() {
        input.accession = resolved.accession.clone();
    }
    if input.sequence.trim().is_empty() {
        input.sequence = resolved.cdna.clone();
    }
    let mut tx = resolved.to_transcript();
    if !input.accession.is_empty() {
        tx.id = input.accession.clone();
    }
    tx.symbol = input.gene_symbol.clone();
    if !req.name.trim().is_empty() {
        tx.name = req.name;
    }
    let cds = req.cds.unwrap_or(resolved.cds);
    design_from_resolved(&input, &tx, cds)
}

fn design_from_sequence(req: DesignRequest) -> Result<DesignResult, Error> {
    let input = req.input;
    match req.cds {
        Some(cds) => {
            let dna = normalize_dna(&input.sequence);
            if dna.is_empty() {
                return Err(Error::Msg("sequence is empty after ACGT normalize".into()));
            }
            let id = if !input.accession.is_empty() {
                input.accession.clone()
            } else {
                input.gene_symbol.clone()
            };
            let name = if !req.name.trim().is_empty() {
                req.name
            } else {
                input.gene_symbol.clone()
            };
            let tx = Transcript {
                id,
                symbol: input.gene_symbol.clone(),
                name,
                length: dna.len() as u32,
                sequence: dna,
            };
            design_from_resolved(&input, &tx, cds)
        }
        None => design_sirnas(&input),
    }
}

fn has_sequence(req: &DesignRequest) -> bool {
    !req.input.sequence.trim().is_empty()
}

fn lookup_symbol(req: &DesignRequest) -> Option<String> {
    let s = req.symbol.trim();
    if !s.is_empty() {
        return Some(s.to_string());
    }
    None
}

fn ensembl_species(explicit: &str, specificity: &str) -> String {
    let e = explicit.trim();
    if !e.is_empty() {
        return e.to_string();
    }
    if specificity.to_ascii_lowercase().starts_with("mouse") {
        "mus_musculus".into()
    } else {
        DEFAULT_SPECIES.into()
    }
}

/// `ENSEMBL_URL` if set and non-empty, else [`ENSEMBL_REST`].
pub fn ensembl_base_url() -> String {
    env_nonempty(ENSEMBL_URL_ENV).unwrap_or_else(|| ENSEMBL_REST.to_string())
}

/// `LISTEN_ADDR` if set and non-empty, else [`DEFAULT_LISTEN_ADDR`].
pub fn listen_addr() -> String {
    env_nonempty(LISTEN_ADDR_ENV).unwrap_or_else(|| DEFAULT_LISTEN_ADDR.to_string())
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            Error::Msg(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Error::EnsemblHttp { status, message } => (
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                message.clone(),
            ),
            Error::OfftargetHttp { status, message } => (
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                message.clone(),
            ),
            Error::Http(m) => (StatusCode::BAD_GATEWAY, m.clone()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_prefers_symbol_field() {
        let req = DesignRequest {
            symbol: "  PCSK9 ".into(),
            name: String::new(),
            species: String::new(),
            cds: None,
            input: DesignInput {
                gene_symbol: "OTHER".into(),
                ..DesignInput::default()
            },
        };
        assert_eq!(lookup_symbol(&req).as_deref(), Some("PCSK9"));
        assert!(!has_sequence(&req));
    }

    #[test]
    fn ensembl_species_from_specificity() {
        assert_eq!(ensembl_species("", "human-230"), DEFAULT_SPECIES);
        assert_eq!(ensembl_species("", "mouse-230"), "mus_musculus");
        assert_eq!(ensembl_species("custom", "mouse-230"), "custom");
    }

    #[test]
    fn allow_origin_defaults_to_local_next() {
        assert_eq!(
            parse_cors_origins(None),
            ParsedCorsOrigins::List(default_cors_origin_list())
        );
    }

    #[test]
    fn allow_origin_star_is_any() {
        assert_eq!(parse_cors_origins(Some("*")), ParsedCorsOrigins::Any);
        assert_eq!(parse_cors_origins(Some(" * ")), ParsedCorsOrigins::Any);
    }

    #[test]
    fn allow_origin_parses_comma_list() {
        assert_eq!(
            parse_cors_origins(Some("https://ui.example, http://127.0.0.1:3001")),
            ParsedCorsOrigins::List(vec![
                "https://ui.example".into(),
                "http://127.0.0.1:3001".into()
            ])
        );
    }
}
