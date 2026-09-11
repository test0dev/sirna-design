//! Ensembl REST client: gene symbol → canonical transcript + RefSeq + sequences.
//!
//! Faithful port of the Node/Bun `resolveBySymbol` helper:
//! lookup symbol → expand gene → pick canonical transcript → RefSeq_mRNA xrefs
//! (MANE Select preferred) → fetch cDNA + CDS → locate CDS on cDNA (1-based closed).

use crate::design::{Cds, Transcript};
use crate::error::Error;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::future::Future;
use std::time::Duration;

/// Ensembl REST origin used by the Node prototype.
pub const ENSEMBL_REST: &str = "https://rest.ensembl.org";
/// Default species for `/lookup/symbol/{species}/{SYM}`.
pub const DEFAULT_SPECIES: &str = "homo_sapiens";

/// Resolved transcript payload that can feed [`crate::design::design_from_resolved`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTarget {
    pub symbol: String,
    pub name: String,
    pub accession: String,
    pub ensembl_transcript: String,
    pub cdna: String,
    pub cds: Cds,
}

impl ResolvedTarget {
    /// Build the design [`Transcript`] (id = RefSeq accession, sequence = cDNA).
    pub fn to_transcript(&self) -> Transcript {
        let id = if self.accession.is_empty() {
            self.ensembl_transcript.clone()
        } else {
            self.accession.clone()
        };
        Transcript {
            id,
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            length: self.cdna.len() as u32,
            sequence: self.cdna.clone(),
        }
    }
}

/// reqwest client pointed at Ensembl (or a test double via [`Self::with_base`]).
#[derive(Debug, Clone)]
pub struct EnsemblClient {
    http: reqwest::Client,
    base: String,
}

impl EnsemblClient {
    pub fn new() -> Result<Self, Error> {
        Self::with_base(ENSEMBL_REST)
    }

    pub fn with_base(base: impl Into<String>) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("sirna-design/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| Error::Http(e.to_string()))?;
        Ok(Self {
            http,
            base: base.into(),
        })
    }

    pub async fn resolve_by_symbol(&self, symbol: &str) -> Result<ResolvedTarget, Error> {
        self.resolve_by_symbol_for(symbol, DEFAULT_SPECIES).await
    }

    pub async fn resolve_by_symbol_for(
        &self,
        symbol: &str,
        species: &str,
    ) -> Result<ResolvedTarget, Error> {
        resolve_with(self, symbol, species).await
    }
}

/// `GET /lookup/symbol/{species}/{SYM}` then the expand / xref / sequence chain.
pub async fn resolve_by_symbol(symbol: &str) -> Result<ResolvedTarget, Error> {
    EnsemblClient::new()?.resolve_by_symbol(symbol).await
}

/// Same as [`resolve_by_symbol`] with an explicit Ensembl species slug.
pub async fn resolve_by_symbol_for(symbol: &str, species: &str) -> Result<ResolvedTarget, Error> {
    EnsemblClient::new()?
        .resolve_by_symbol_for(symbol, species)
        .await
}

pub(crate) trait EnsemblGet: Sync {
    fn get_json(&self, path: &str) -> impl Future<Output = Result<Value, Error>> + Send;
}

impl EnsemblGet for EnsemblClient {
    async fn get_json(&self, path: &str) -> Result<Value, Error> {
        const ATTEMPTS: u32 = 4;
        let mut last = None;
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                let backoff = Duration::from_millis(300 * (1 << (attempt - 1)));
                tracing::debug!(path, attempt, ?backoff, "ensembl retry");
                tokio::time::sleep(backoff).await;
            }
            match self.get_json_once(path).await {
                Ok(v) => return Ok(v),
                Err(e) if retryable(&e) && attempt + 1 < ATTEMPTS => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or_else(|| Error::Http(format!("{path}: retry exhausted"))))
    }
}

impl EnsemblClient {
    /// `GET /sequence/id/{id}?type=cdna` → `(header, sequence)`.
    pub async fn fetch_cdna(&self, id: &str) -> Result<(String, String), Error> {
        let id = id.trim();
        if id.is_empty() {
            return Err(Error::Msg("accession is required".into()));
        }
        let json = self.get_json(&sequence_path(id, "cdna")).await?;
        let seq = parse_seq(&json)?;
        if seq.is_empty() {
            return Err(Error::Msg(format!(
                "Ensembl cDNA sequence is empty for {id}"
            )));
        }
        let header = json
            .get("desc")
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| json.get("id").and_then(|s| s.as_str()))
            .unwrap_or(id)
            .to_string();
        Ok((header, seq))
    }

    async fn get_json_once(&self, path: &str) -> Result<Value, Error> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        tracing::debug!(%url, "ensembl GET");
        let resp = self
            .http
            .get(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| Error::Http(format!("{path}: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Http(format!("{path}: {e}")))?;
        if !status.is_success() {
            let message = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
                .filter(|s| !s.is_empty())
                .unwrap_or(body);
            return Err(Error::EnsemblHttp {
                status: status.as_u16(),
                message,
            });
        }
        serde_json::from_str(&body).map_err(|e| Error::Msg(format!("invalid Ensembl JSON: {e}")))
    }
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
        Error::EnsemblHttp { status, .. } => matches!(*status, 429 | 500 | 502 | 503 | 504),
        _ => false,
    }
}

pub(crate) async fn resolve_with<C: EnsemblGet>(
    client: &C,
    symbol: &str,
    species: &str,
) -> Result<ResolvedTarget, Error> {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return Err(Error::Msg("gene symbol is required".into()));
    }
    let species = {
        let s = species.trim();
        if s.is_empty() {
            DEFAULT_SPECIES
        } else {
            s
        }
    };

    let lookup: LookupGene = deserialize(
        client.get_json(&lookup_symbol_path(species, &sym)).await?,
        "lookup/symbol",
    )?;
    if lookup.id.is_empty() {
        return Err(Error::Msg(format!(
            "Ensembl lookup returned no id for {sym}"
        )));
    }

    let gene: LookupGene = deserialize(
        client.get_json(&lookup_id_expand_path(&lookup.id)).await?,
        "lookup/id",
    )?;

    let tx = pick_canonical(&gene)?;
    let tx_id = tx.id.clone();

    let accession = match fetch_refseq_accession(client, &tx_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => tx_id.clone(),
        Err(e) => {
            tracing::warn!(
                tx_id,
                error = %e,
                "xrefs failed; falling back to Ensembl transcript id"
            );
            tx_id.clone()
        }
    };

    let cdna_json = client.get_json(&sequence_path(&tx_id, "cdna")).await?;
    let cds_json = client.get_json(&sequence_path(&tx_id, "cds")).await?;
    let cdna = parse_seq(&cdna_json)?;
    let cds_seq = parse_seq(&cds_json)?;
    if cdna.is_empty() {
        return Err(Error::Msg(format!(
            "Ensembl cDNA sequence is empty for {tx_id}"
        )));
    }

    let symbol_out = gene
        .display_name
        .as_deref()
        .or(lookup.display_name.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or(sym.as_str())
        .to_string();
    let raw_name = gene
        .description
        .as_deref()
        .or(lookup.description.as_deref())
        .unwrap_or("");

    Ok(ResolvedTarget {
        symbol: symbol_out,
        name: clean_description(raw_name),
        accession,
        ensembl_transcript: tx_id,
        cds: locate_cds(&cdna, &cds_seq),
        cdna,
    })
}

/// Strip the Ensembl ` [Source:...]` suffix from a gene description.
pub fn clean_description(desc: &str) -> String {
    let cut = desc
        .find(" [Source:")
        .or_else(|| desc.find("[Source:"))
        .unwrap_or(desc.len());
    desc[..cut].trim().to_string()
}

/// Locate `cds` inside `cdna` as a 1-based closed interval.
///
/// When `cds` is missing from `cdna`, the whole transcript is used.
pub fn locate_cds(cdna: &str, cds: &str) -> Cds {
    if !cds.is_empty() {
        if let Some(idx) = cdna.find(cds) {
            return Cds {
                start: (idx + 1) as u32,
                end: (idx + cds.len()) as u32,
            };
        }
    }
    Cds {
        start: 1,
        end: cdna.len() as u32,
    }
}

/// `is_canonical` flag → `canonical_transcript` id → longest protein_coding → longest.
pub fn pick_canonical(gene: &LookupGene) -> Result<&LookupTranscript, Error> {
    if gene.transcripts.is_empty() {
        return Err(Error::Msg(format!("no transcripts for gene {}", gene.id)));
    }
    if let Some(tx) = gene.transcripts.iter().find(|t| t.is_canonical) {
        return Ok(tx);
    }
    if let Some(canon) = gene.canonical_transcript.as_deref() {
        if let Some(tx) = gene
            .transcripts
            .iter()
            .find(|t| transcript_id_matches(t, canon))
        {
            return Ok(tx);
        }
    }
    let coding: Vec<&LookupTranscript> = gene
        .transcripts
        .iter()
        .filter(|t| t.biotype.as_deref() == Some("protein_coding"))
        .collect();
    let pool = if coding.is_empty() {
        gene.transcripts.iter().collect::<Vec<_>>()
    } else {
        coding
    };
    Ok(pool
        .into_iter()
        .max_by_key(|t| transcript_len(t))
        .expect("non-empty transcript pool"))
}

async fn fetch_refseq_accession<C: EnsemblGet>(
    client: &C,
    tx_id: &str,
) -> Result<Option<String>, Error> {
    let value = client.get_json(&xrefs_path(tx_id)).await?;
    Ok(pick_refseq(&parse_xrefs(value)))
}

/// Best-effort parse of `/xrefs/id` JSON (array, wrapped object, or junk).
pub fn parse_xrefs(value: Value) -> Vec<Xref> {
    let items = match value {
        Value::Array(a) => a,
        Value::Object(map) => map
            .get("xrefs")
            .or_else(|| map.get("Xrefs"))
            .cloned()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        _ => return Vec::new(),
    };
    items
        .into_iter()
        .filter(|v| !v.is_null())
        .filter_map(|v| serde_json::from_value::<Xref>(v).ok())
        .collect()
}

/// First `RefSeq_mRNA` xref, preferring one whose text mentions MANE Select.
pub fn pick_refseq(xrefs: &[Xref]) -> Option<String> {
    let mrna: Vec<&Xref> = xrefs.iter().filter(|x| x.dbname == "RefSeq_mRNA").collect();
    let mane = mrna.iter().copied().find(|x| x.is_mane_select());
    let hit = mane.or_else(|| mrna.first().copied())?;
    hit.accession()
}

/// Gene object from `/lookup/symbol` or `/lookup/id?expand=1`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LookupGene {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub canonical_transcript: Option<String>,
    #[serde(default, rename = "Transcript")]
    pub transcripts: Vec<LookupTranscript>,
}

/// Transcript object nested under an expanded gene lookup.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LookupTranscript {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub biotype: Option<String>,
    #[serde(default, deserialize_with = "de_is_canonical")]
    pub is_canonical: bool,
    #[serde(default)]
    pub length: Option<i64>,
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
    #[serde(default, rename = "Translation")]
    pub translation: Option<LookupTranslation>,
    #[serde(default, rename = "Exon")]
    pub exons: Vec<LookupExon>,
}

/// Translation block on an expanded transcript (length = protein aa).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LookupTranslation {
    #[serde(default)]
    pub length: Option<i64>,
}

/// Exon block used as a fallback length signal.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LookupExon {
    #[serde(default)]
    pub length: Option<i64>,
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
}

/// One `/xrefs/id/{tx}` row.
///
/// Ensembl occasionally emits nulls, extra fields, or mixed-type cells.
/// Unknown fields are ignored; nulls become empty / `None`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Xref {
    #[serde(default, deserialize_with = "de_flex_string")]
    pub dbname: String,
    #[serde(default, deserialize_with = "de_flex_opt_string")]
    pub display_id: Option<String>,
    #[serde(default, deserialize_with = "de_flex_opt_string")]
    pub primary_id: Option<String>,
    #[serde(default, deserialize_with = "de_flex_opt_string")]
    pub info_text: Option<String>,
    #[serde(default, deserialize_with = "de_flex_opt_string")]
    pub info_type: Option<String>,
    #[serde(default, deserialize_with = "de_flex_opt_string")]
    pub description: Option<String>,
}

impl Xref {
    fn is_mane_select(&self) -> bool {
        [&self.info_text, &self.description, &self.info_type]
            .into_iter()
            .filter_map(|s| s.as_deref())
            .any(|s| s.to_ascii_lowercase().contains("mane select"))
    }

    fn accession(&self) -> Option<String> {
        self.display_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(self.primary_id.as_deref().filter(|s| !s.is_empty()))
            .map(|s| s.to_string())
    }
}

fn lookup_symbol_path(species: &str, symbol: &str) -> String {
    format!(
        "/lookup/symbol/{}/{}",
        path_segment(species),
        path_segment(symbol)
    )
}

fn lookup_id_expand_path(id: &str) -> String {
    format!("/lookup/id/{}?expand=1", path_segment(id))
}

fn xrefs_path(id: &str) -> String {
    format!("/xrefs/id/{}", path_segment(id))
}

fn sequence_path(id: &str, kind: &str) -> String {
    format!(
        "/sequence/id/{}?type={}",
        path_segment(id),
        path_segment(kind)
    )
}

fn path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn transcript_id_matches(tx: &LookupTranscript, canon: &str) -> bool {
    if tx.id == canon {
        return true;
    }
    if let Some(v) = tx.version {
        if format!("{}.{}", tx.id, v) == canon {
            return true;
        }
    }
    canon.starts_with(&tx.id) && canon.as_bytes().get(tx.id.len()) == Some(&b'.')
}

fn transcript_len(tx: &LookupTranscript) -> i64 {
    if let Some(len) = tx.length {
        return len;
    }
    if !tx.exons.is_empty() {
        return tx
            .exons
            .iter()
            .map(|e| {
                e.length.unwrap_or_else(|| match (e.start, e.end) {
                    (Some(s), Some(end)) => (end - s).abs() + 1,
                    _ => 0,
                })
            })
            .sum();
    }
    match (tx.start, tx.end) {
        (Some(s), Some(e)) => (e - s).abs() + 1,
        _ => tx.translation.as_ref().and_then(|t| t.length).unwrap_or(0),
    }
}

fn parse_seq(v: &Value) -> Result<String, Error> {
    if let Some(s) = v.as_str() {
        return Ok(s.to_string());
    }
    v.get("seq")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Msg("Ensembl sequence response missing seq".into()))
}

fn deserialize<T: for<'de> Deserialize<'de>>(value: Value, what: &str) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|e| Error::Msg(format!("invalid Ensembl {what}: {e}")))
}

fn de_flex_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(string_from_value(Value::deserialize(deserializer)?))
}

fn de_flex_opt_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = string_from_value(Value::deserialize(deserializer)?);
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

fn string_from_value(v: Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn de_is_canonical<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct Flag;
    impl<'de> Visitor<'de> for Flag {
        type Value = bool;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a boolean or 0/1 canonical flag")
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
            Ok(v != 0)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
            Ok(v != 0)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<bool, E> {
            match v.trim() {
                "1" | "true" | "True" | "TRUE" => Ok(true),
                "0" | "false" | "False" | "FALSE" | "" => Ok(false),
                other => Err(E::custom(format!("invalid is_canonical {other:?}"))),
            }
        }
        fn visit_unit<E: de::Error>(self) -> Result<bool, E> {
            Ok(false)
        }
    }
    deserializer.deserialize_any(Flag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    struct MapGet(HashMap<String, Value>);

    impl EnsemblGet for MapGet {
        async fn get_json(&self, path: &str) -> Result<Value, Error> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| Error::Msg(format!("unexpected Ensembl path {path}")))
        }
    }

    fn tx(id: &str, canonical: bool, biotype: &str, length: i64) -> LookupTranscript {
        LookupTranscript {
            id: id.into(),
            is_canonical: canonical,
            biotype: Some(biotype.into()),
            length: Some(length),
            ..LookupTranscript::default()
        }
    }

    #[test]
    fn retryable_timeouts_and_transient_http() {
        assert!(retryable(&Error::Http(
            "error sending request for url (timed out)".into()
        )));
        assert!(retryable(&Error::EnsemblHttp {
            status: 503,
            message: "unavailable".into()
        }));
        assert!(!retryable(&Error::EnsemblHttp {
            status: 400,
            message: "bad symbol".into()
        }));
        assert!(!retryable(&Error::Msg("no transcripts".into())));
    }

    #[test]
    fn clean_description_strips_source_suffix() {
        assert_eq!(
            clean_description(
                "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]"
            ),
            "proprotein convertase subtilisin/kexin type 9"
        );
        assert_eq!(clean_description("plain name"), "plain name");
        assert_eq!(clean_description("  spaced  "), "spaced");
        assert_eq!(clean_description("[Source:only]"), "");
    }

    #[test]
    fn locate_cds_is_one_based_closed_or_whole_transcript() {
        let cdna = "AAAATGCCCTAA";
        let cds = "ATGCCC";
        assert_eq!(locate_cds(cdna, cds), Cds { start: 4, end: 9 });
        assert_eq!(locate_cds(cdna, "GGG"), Cds { start: 1, end: 12 });
        assert_eq!(locate_cds(cdna, ""), Cds { start: 1, end: 12 });
    }

    #[test]
    fn pick_canonical_prefers_flag_then_id_then_longest_coding() {
        let flagged = LookupGene {
            id: "ENSG1".into(),
            transcripts: vec![
                tx("ENST_long", false, "protein_coding", 9000),
                tx("ENST_canon", true, "protein_coding", 100),
            ],
            ..LookupGene::default()
        };
        assert_eq!(pick_canonical(&flagged).unwrap().id, "ENST_canon");

        let by_id = LookupGene {
            id: "ENSG1".into(),
            canonical_transcript: Some("ENST_v.5".into()),
            transcripts: vec![
                LookupTranscript {
                    id: "ENST_v".into(),
                    version: Some(5),
                    biotype: Some("protein_coding".into()),
                    length: Some(10),
                    ..LookupTranscript::default()
                },
                tx("ENST_other", false, "protein_coding", 9999),
            ],
            ..LookupGene::default()
        };
        assert_eq!(pick_canonical(&by_id).unwrap().id, "ENST_v");

        let longest_coding = LookupGene {
            id: "ENSG1".into(),
            transcripts: vec![
                tx("ENST_nmd", false, "nonsense_mediated_decay", 8000),
                tx("ENST_short", false, "protein_coding", 100),
                tx("ENST_long", false, "protein_coding", 400),
            ],
            ..LookupGene::default()
        };
        assert_eq!(pick_canonical(&longest_coding).unwrap().id, "ENST_long");

        let longest_any = LookupGene {
            id: "ENSG1".into(),
            transcripts: vec![
                tx("ENST_a", false, "retained_intron", 50),
                tx("ENST_b", false, "lncRNA", 80),
            ],
            ..LookupGene::default()
        };
        assert_eq!(pick_canonical(&longest_any).unwrap().id, "ENST_b");

        let empty = LookupGene {
            id: "ENSG1".into(),
            ..LookupGene::default()
        };
        assert!(pick_canonical(&empty)
            .unwrap_err()
            .to_string()
            .contains("no transcripts"));
    }

    #[test]
    fn pick_refseq_prefers_mane_select_else_first() {
        let xrefs = vec![
            Xref {
                dbname: "HGNC".into(),
                display_id: Some("PCSK9".into()),
                ..Xref::default()
            },
            Xref {
                dbname: "RefSeq_mRNA".into(),
                display_id: Some("NM_first.1".into()),
                info_text: Some("Generated via otherfeatures".into()),
                ..Xref::default()
            },
            Xref {
                dbname: "RefSeq_mRNA".into(),
                display_id: Some("NM_mane.4".into()),
                info_text: Some("MANE Select".into()),
                ..Xref::default()
            },
        ];
        assert_eq!(pick_refseq(&xrefs).as_deref(), Some("NM_mane.4"));

        let no_mane = vec![
            Xref {
                dbname: "RefSeq_mRNA".into(),
                display_id: Some("NM_001407241.1".into()),
                info_text: Some("Generated via otherfeatures".into()),
                ..Xref::default()
            },
            Xref {
                dbname: "RefSeq_mRNA".into(),
                display_id: Some("NM_174936.4".into()),
                info_text: Some("Generated via otherfeatures".into()),
                ..Xref::default()
            },
        ];
        assert_eq!(pick_refseq(&no_mane).as_deref(), Some("NM_001407241.1"));
        assert_eq!(pick_refseq(&[]), None);
    }

    #[test]
    fn is_canonical_deserializes_int_and_bool() {
        let t: LookupTranscript = serde_json::from_value(json!({
            "id": "ENST1",
            "is_canonical": 1
        }))
        .unwrap();
        assert!(t.is_canonical);
        let t: LookupTranscript = serde_json::from_value(json!({
            "id": "ENST2",
            "is_canonical": 0
        }))
        .unwrap();
        assert!(!t.is_canonical);
        let t: LookupTranscript = serde_json::from_value(json!({
            "id": "ENST3",
            "is_canonical": true
        }))
        .unwrap();
        assert!(t.is_canonical);
    }

    fn mock_pcsk9(cdna: &str, cds: &str, mane: bool) -> MapGet {
        let x0 = json!({
            "dbname": "RefSeq_mRNA",
            "display_id": "NM_001407241.1",
            "primary_id": "NM_001407241",
            "info_text": "Generated via otherfeatures"
        });
        let x1 = json!({
            "dbname": "RefSeq_mRNA",
            "display_id": "NM_174936.4",
            "primary_id": "NM_174936",
            "info_text": if mane { "MANE Select" } else { "Generated via otherfeatures" }
        });
        let routes = HashMap::from([
            (
                "/lookup/symbol/homo_sapiens/PCSK9".into(),
                json!({
                    "id": "ENSG00000169174",
                    "display_name": "PCSK9",
                    "description": "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]"
                }),
            ),
            (
                "/lookup/id/ENSG00000169174?expand=1".into(),
                json!({
                    "id": "ENSG00000169174",
                    "display_name": "PCSK9",
                    "description": "proprotein convertase subtilisin/kexin type 9 [Source:HGNC Symbol;Acc:HGNC:20001]",
                    "canonical_transcript": "ENST00000302118.5",
                    "Transcript": [
                        {
                            "id": "ENST00000710286",
                            "is_canonical": 0,
                            "biotype": "protein_coding",
                            "length": 3729
                        },
                        {
                            "id": "ENST00000302118",
                            "version": 5,
                            "is_canonical": 1,
                            "biotype": "protein_coding",
                            "length": 3637
                        }
                    ]
                }),
            ),
            ("/xrefs/id/ENST00000302118".into(), json!([x0, x1])),
            (
                "/sequence/id/ENST00000302118?type=cdna".into(),
                json!({ "id": "ENST00000302118", "seq": cdna }),
            ),
            (
                "/sequence/id/ENST00000302118?type=cds".into(),
                json!({ "id": "ENST00000302118", "seq": cds }),
            ),
        ]);
        MapGet(routes)
    }

    #[tokio::test]
    async fn resolve_with_mock_http_pcsk9_shape() {
        let cdna = "AAAATGCCCTAA";
        let cds = "ATGCCC";
        let got = resolve_with(&mock_pcsk9(cdna, cds, true), " pcsk9 ", DEFAULT_SPECIES)
            .await
            .expect("resolve");
        assert_eq!(got.symbol, "PCSK9");
        assert_eq!(got.name, "proprotein convertase subtilisin/kexin type 9");
        assert_eq!(got.accession, "NM_174936.4");
        assert_eq!(got.ensembl_transcript, "ENST00000302118");
        assert_eq!(got.cdna, cdna);
        assert_eq!(got.cds, Cds { start: 4, end: 9 });

        let tx = got.to_transcript();
        assert_eq!(tx.id, "NM_174936.4");
        assert_eq!(tx.symbol, "PCSK9");
        assert_eq!(tx.name, got.name);
        assert_eq!(tx.sequence, cdna);
        assert_eq!(tx.length, cdna.len() as u32);
    }

    #[test]
    fn parse_xrefs_tolerates_nulls_extras_and_wrapped_object() {
        let weird = json!([
            null,
            { "dbname": null, "display_id": null, "mystery": { "n": 1 } },
            {
                "dbname": "RefSeq_mRNA",
                "display_id": "NM_174936.4",
                "primary_id": null,
                "info_text": "MANE Select",
                "synonyms": [null, "x"]
            }
        ]);
        let got = parse_xrefs(weird);
        assert_eq!(pick_refseq(&got).as_deref(), Some("NM_174936.4"));

        let wrapped = json!({
            "xrefs": [{ "dbname": "RefSeq_mRNA", "display_id": "NM_1.1" }]
        });
        assert_eq!(
            pick_refseq(&parse_xrefs(wrapped)).as_deref(),
            Some("NM_1.1")
        );
        assert!(parse_xrefs(json!({"error": "nope"})).is_empty());
    }

    #[tokio::test]
    async fn resolve_weird_xrefs_payload_still_returns_cdna() {
        let cdna = "AAAATGCCCTAA";
        let cds = "ATGCCC";
        let mut api = mock_pcsk9(cdna, cds, true);
        api.0.insert(
            "/xrefs/id/ENST00000302118".into(),
            json!([
                null,
                { "dbname": null, "extra": true },
                {
                    "dbname": "RefSeq_mRNA",
                    "display_id": "NM_174936.4",
                    "info_text": null,
                    "synonyms": [null]
                }
            ]),
        );
        let got = resolve_with(&api, "PCSK9", DEFAULT_SPECIES)
            .await
            .expect("resolve");
        assert_eq!(got.accession, "NM_174936.4");
        assert_eq!(got.ensembl_transcript, "ENST00000302118");
        assert_eq!(got.cdna, cdna);
        assert_eq!(got.cds, Cds { start: 4, end: 9 });
    }

    #[tokio::test]
    async fn resolve_xrefs_http_error_falls_back_to_enst() {
        let cdna = "AAAATGCCCTAA";
        let mut api = mock_pcsk9(cdna, "ATGCCC", true);
        api.0.remove("/xrefs/id/ENST00000302118");
        let got = resolve_with(&api, "PCSK9", DEFAULT_SPECIES)
            .await
            .expect("resolve despite missing xrefs");
        assert_eq!(got.accession, "ENST00000302118");
        assert_eq!(got.cdna, cdna);
    }

    #[tokio::test]
    async fn resolve_uppercases_symbol_and_falls_back_accession() {
        let mut api = mock_pcsk9("ATGC", "ATG", false);
        api.0.insert(
            "/xrefs/id/ENST00000302118".into(),
            json!([{ "dbname": "HGNC", "display_id": "PCSK9" }]),
        );
        let got = resolve_with(&api, "pcsk9", DEFAULT_SPECIES)
            .await
            .expect("resolve");
        assert_eq!(got.accession, "ENST00000302118");
    }

    #[tokio::test]
    async fn resolve_rejects_empty_symbol() {
        let api = MapGet(HashMap::new());
        let err = resolve_with(&api, "  ", DEFAULT_SPECIES).await.unwrap_err();
        assert!(err.to_string().contains("symbol is required"));
    }

    #[tokio::test]
    async fn resolved_target_feeds_design_from_resolved() {
        let cdna = "NNNNATGAAAAAAAAAAAAAAAAAAAAAA";
        let cds = "ATGAAAAAAAAAAAAAAAAAAAAAA";
        let got = resolve_with(&mock_pcsk9(cdna, cds, false), "PCSK9", "")
            .await
            .unwrap();
        let mut input = crate::design::default_input();
        input.specificity = "none".into();
        input.gc_min = 0.0;
        input.gc_max = 100.0;
        input.avoid_contiguous_gc = false;
        input.avoid_contiguous_at = false;
        input.seed_tm_max = 100.0;
        let designed = crate::design::design_from_resolved(&input, &got.to_transcript(), got.cds)
            .expect("design");
        assert_eq!(designed.transcript.id, "NM_001407241.1");
        assert_eq!(designed.transcript.symbol, "PCSK9");
        assert_eq!(designed.transcript.sequence, got.cdna);
        assert_eq!(designed.cds, got.cds);
    }
}
