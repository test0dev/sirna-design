//! Gene-symbol resolve and accession retrieve (Ensembl / NCBI) with redb cache.

use crate::design::Cds;
use crate::ensembl::{EnsemblClient, ResolvedTarget, DEFAULT_SPECIES};
use crate::error::Error;
use crate::resolve_cache::{accession_key, symbol_key, ResolveCache};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// NCBI E-utilities efetch (fasta).
pub const NCBI_EFETCH: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi";

/// `GET /v1/resolve` body (camelCase).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveResponse {
    pub symbol: String,
    pub name: String,
    pub accession: String,
    pub ensembl_transcript: String,
    pub species: String,
    pub cds: Cds,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sequence: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub length: u32,
    #[serde(default)]
    pub cached: bool,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl ResolveResponse {
    pub fn from_resolved(r: ResolvedTarget, species: &str, include_sequence: bool) -> Self {
        let length = r.cdna.len() as u32;
        Self {
            symbol: r.symbol,
            name: r.name,
            accession: r.accession,
            ensembl_transcript: r.ensembl_transcript,
            species: species.to_string(),
            cds: r.cds,
            sequence: if include_sequence {
                r.cdna
            } else {
                String::new()
            },
            length: if include_sequence { length } else { 0 },
            cached: false,
        }
    }

    pub fn to_resolved_target(&self) -> Result<ResolvedTarget, Error> {
        if self.sequence.is_empty() {
            return Err(Error::Msg("resolved target has no sequence".into()));
        }
        Ok(ResolvedTarget {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            accession: self.accession.clone(),
            ensembl_transcript: self.ensembl_transcript.clone(),
            cdna: self.sequence.clone(),
            cds: self.cds,
        })
    }
}

/// `GET /v1/retrieve` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrieveResponse {
    pub accession: String,
    pub header: String,
    pub sequence: String,
    pub length: u32,
    #[serde(default)]
    pub cached: bool,
}

/// Query string for `GET /v1/resolve`.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveQuery {
    pub symbol: String,
    #[serde(default)]
    pub species: String,
    #[serde(default = "default_true", deserialize_with = "de_boolish")]
    pub include_sequence: bool,
}

fn default_true() -> bool {
    true
}

/// Query string for `GET /v1/retrieve`.
#[derive(Debug, Clone, Deserialize)]
pub struct RetrieveQuery {
    pub accession: String,
}

/// NCBI fasta client (origin or full efetch URL).
#[derive(Debug, Clone)]
pub struct NcbiClient {
    http: reqwest::Client,
    base: String,
}

impl NcbiClient {
    pub fn new() -> Result<Self, Error> {
        Self::with_base(NCBI_EFETCH)
    }

    pub fn with_base(base: impl Into<String>) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("sirna-design/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| Error::Http(e.to_string()))?;
        Ok(Self {
            http,
            base: base.into(),
        })
    }

    pub async fn fetch_fasta(&self, accession: &str) -> Result<RetrieveResponse, Error> {
        let id = accession.trim();
        if id.is_empty() {
            return Err(Error::Msg("Enter an accession number first.".into()));
        }
        let url = ncbi_efetch_url(&self.base, id)?;
        let resp = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "text/plain")
            .send()
            .await
            .map_err(|e| Error::Http(format!("ncbi: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Http(format!("ncbi: {e}")))?;
        if !status.is_success() {
            return Err(Error::Http(format!(
                "NCBI retrieve returned HTTP {}.",
                status.as_u16()
            )));
        }
        parse_retrieve_fasta(&text, id)
    }
}

fn ncbi_efetch_url(base: &str, id: &str) -> Result<reqwest::Url, Error> {
    let mut url = reqwest::Url::parse(base).map_err(|e| Error::Http(format!("ncbi url: {e}")))?;
    url.query_pairs_mut()
        .append_pair("db", "nuccore")
        .append_pair("id", id)
        .append_pair("rettype", "fasta")
        .append_pair("retmode", "text")
        .append_pair("tool", "sirna-design");
    Ok(url)
}

/// Resolve symbol via Ensembl, with redb read-through cache.
pub async fn resolve_symbol(
    ensembl: &EnsemblClient,
    cache: &ResolveCache,
    symbol: &str,
    species: &str,
    include_sequence: bool,
) -> Result<ResolveResponse, Error> {
    let species = {
        let s = species.trim();
        if s.is_empty() {
            DEFAULT_SPECIES
        } else {
            s
        }
    };
    let key = symbol_key(species, symbol, include_sequence);
    match cache.get_resolve::<ResolveResponse>(&key) {
        Ok(Some(mut hit)) => {
            hit.cached = true;
            return Ok(hit);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "resolve cache read failed"),
    }

    let resolved = ensembl.resolve_by_symbol_for(symbol, species).await?;
    let mut resp = ResolveResponse::from_resolved(resolved, species, include_sequence);
    resp.cached = false;
    if let Err(e) = cache.put_resolve(&key, &resp) {
        tracing::warn!(error = %e, "resolve cache write failed");
    }
    Ok(resp)
}

/// Fetch cDNA / FASTA for an accession, with redb read-through cache.
pub async fn retrieve_accession(
    ensembl: &EnsemblClient,
    ncbi: &NcbiClient,
    cache: &ResolveCache,
    accession: &str,
) -> Result<RetrieveResponse, Error> {
    let acc = accession.trim();
    if acc.is_empty() {
        return Err(Error::Msg("Enter an accession number first.".into()));
    }
    let key = accession_key(acc);
    match cache.get_retrieve::<RetrieveResponse>(&key) {
        Ok(Some(mut hit)) => {
            hit.cached = true;
            return Ok(hit);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "retrieve cache read failed"),
    }

    let mut got = if acc.to_ascii_uppercase().starts_with("ENST") {
        let (header, sequence) = ensembl.fetch_cdna(acc).await?;
        let length = sequence.len() as u32;
        RetrieveResponse {
            accession: acc.to_string(),
            header,
            sequence,
            length,
            cached: false,
        }
    } else {
        ncbi.fetch_fasta(acc).await?
    };
    got.cached = false;
    if let Err(e) = cache.put_retrieve(&key, &got) {
        tracing::warn!(error = %e, "retrieve cache write failed");
    }
    Ok(got)
}

/// Port of `web/lib/ncbi-retrieve.ts` `parseRetrieveFasta`.
pub fn parse_retrieve_fasta(text: &str, requested: &str) -> Result<RetrieveResponse, Error> {
    let body = text.trim_start_matches('\u{feff}').trim();
    if body.is_empty() {
        return Err(Error::Msg(format!("No sequence returned for {requested}.")));
    }
    if is_retrieve_error(body) {
        return Err(Error::Msg(format!("Accession not found: {requested}.")));
    }
    if !body.starts_with('>') {
        return Err(Error::Msg(format!(
            "NCBI did not return FASTA for {requested}."
        )));
    }

    let header_line = body
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches('>')
        .trim();
    let seq_len = fasta_base_count(body);
    if seq_len == 0 {
        return Err(Error::Msg(format!(
            "Retrieved FASTA for {requested} has no bases."
        )));
    }
    let accession = extract_accession(header_line).unwrap_or_else(|| requested.to_string());
    let mut fasta = body.replace("\r\n", "\n");
    if !fasta.ends_with('\n') {
        fasta.push('\n');
    }
    Ok(RetrieveResponse {
        accession,
        header: if header_line.is_empty() {
            requested.to_string()
        } else {
            header_line.to_string()
        },
        sequence: fasta,
        length: seq_len as u32,
        cached: false,
    })
}

fn fasta_base_count(text: &str) -> usize {
    text.lines()
        .filter(|l| !l.starts_with('>'))
        .flat_map(|l| l.chars())
        .filter(|c| matches!(c.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T' | 'U' | 'N'))
        .count()
}

fn extract_accession(header: &str) -> Option<String> {
    let bytes = header.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &header[i..];
        if let Some(hit) = rest
            .find("NM_")
            .into_iter()
            .chain(rest.find("NR_"))
            .chain(rest.find("XM_"))
            .chain(rest.find("XR_"))
            .chain(rest.find("nm_"))
            .chain(rest.find("xm_"))
            .min()
        {
            let start = i + hit;
            let tail = &header[start..];
            let end = tail
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .unwrap_or(tail.len());
            let token = &tail[..end];
            if token.len() > 3 {
                return Some(token.to_string());
            }
            i = start + 1;
        } else {
            break;
        }
    }
    None
}

fn is_retrieve_error(text: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    text.trim().to_ascii_lowercase().starts_with("error:")
        || compact.starts_with("error:")
        || compact.starts_with("<?xml")
        || compact == "notfound."
        || compact.contains("failedtounderstandid")
}

fn de_boolish<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer).map_err(de::Error::custom)?;
    match v {
        Value::Bool(b) => Ok(b),
        Value::Number(n) => Ok(n.as_i64().unwrap_or(1) != 0),
        Value::String(s) => Ok(match s.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "no" | "off" => false,
            _ => true,
        }),
        Value::Null => Ok(true),
        _ => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fasta_nm_fixture_shape() {
        let fasta = ">NM_012131.3 Homo sapiens claudin 17 (CLDN17), mRNA\nATGC\nTAAA\n";
        let got = parse_retrieve_fasta(fasta, "NM_012131").expect("fasta");
        assert_eq!(got.accession, "NM_012131.3");
        assert!(got.header.contains("CLDN17"));
        assert_eq!(got.length, 8);
        assert!(got.sequence.starts_with(">NM_012131.3 "));
        assert!(got.sequence.ends_with('\n'));
        assert!(!got.cached);
    }

    #[test]
    fn parse_fasta_rejects_errors() {
        assert!(parse_retrieve_fasta("Error: Invalid uid X", "X")
            .unwrap_err()
            .to_string()
            .contains("Accession not found"));
        assert!(parse_retrieve_fasta("not found.", "NM_0")
            .unwrap_err()
            .to_string()
            .contains("Accession not found"));
        assert!(parse_retrieve_fasta("ATGC", "NM_1")
            .unwrap_err()
            .to_string()
            .contains("did not return FASTA"));
    }

    #[test]
    fn from_resolved_omits_sequence_when_asked() {
        let r = ResolvedTarget {
            symbol: "PCSK9".into(),
            name: "n".into(),
            accession: "NM_1".into(),
            ensembl_transcript: "ENST1".into(),
            cdna: "ATGC".into(),
            cds: Cds { start: 1, end: 4 },
        };
        let full = ResolveResponse::from_resolved(r.clone(), "homo_sapiens", true);
        assert_eq!(full.sequence, "ATGC");
        assert_eq!(full.length, 4);
        let slim = ResolveResponse::from_resolved(r, "homo_sapiens", false);
        assert!(slim.sequence.is_empty());
        assert_eq!(slim.length, 0);
        assert_eq!(slim.cds, Cds { start: 1, end: 4 });
        let json = serde_json::to_value(&slim).unwrap();
        assert!(json.get("sequence").is_none());
        assert!(json.get("length").is_none());
        assert_eq!(json["ensemblTranscript"], "ENST1");
    }
}
