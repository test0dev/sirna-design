//! Persistent resolve / retrieve cache (redb).
//!
//! Two tables, JSON values, keys:
//! - symbol: `v1|symbol|{species}|{SYMBOL_UPPER}|seq={0|1}`
//! - accession: `v1|accession|{ACCESSION_UPPER}`
//!
//! The `cached` flag is applied on the read path and is not relied on in storage.

use crate::error::Error;
use redb::{Database, TableDefinition};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Env override for the on-disk cache path.
pub const REDB_PATH_ENV: &str = "REDB_PATH";
/// Default file when [`REDB_PATH_ENV`] is unset.
pub const DEFAULT_REDB_PATH: &str = "data/resolve-cache.redb";

const SYMBOL_TABLE: TableDefinition<&str, &str> = TableDefinition::new("symbol_resolve");
const ACCESSION_TABLE: TableDefinition<&str, &str> = TableDefinition::new("accession_retrieve");

/// Open redb handle shared across the API process.
#[derive(Clone)]
pub struct ResolveCache {
    db: Arc<Database>,
}

impl ResolveCache {
    /// `REDB_PATH` if set, else [`DEFAULT_REDB_PATH`].
    pub fn from_env() -> Result<Self, Error> {
        let path = std::env::var(REDB_PATH_ENV)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_REDB_PATH.to_string());
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Msg(format!("resolve cache mkdir: {e}")))?;
            }
        }
        let db = Database::create(path).map_err(cache_err)?;
        init_tables(&db)?;
        tracing::info!(path = %path.display(), "resolve cache open");
        Ok(Self { db: Arc::new(db) })
    }

    /// Process-local cache (tests / dummy API state).
    pub fn in_memory() -> Result<Self, Error> {
        let db = Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .map_err(cache_err)?;
        init_tables(&db)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn get_resolve<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        self.get(SYMBOL_TABLE, key)
    }

    pub fn put_resolve<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Error> {
        self.put(SYMBOL_TABLE, key, value)
    }

    pub fn get_retrieve<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        self.get(ACCESSION_TABLE, key)
    }

    pub fn put_retrieve<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Error> {
        self.put(ACCESSION_TABLE, key, value)
    }

    fn get<T: DeserializeOwned>(
        &self,
        table: TableDefinition<&str, &str>,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let txn = self.db.begin_read().map_err(cache_err)?;
        let table = txn.open_table(table).map_err(cache_err)?;
        let Some(guard) = table.get(key).map_err(cache_err)? else {
            return Ok(None);
        };
        let parsed = serde_json::from_str(guard.value())
            .map_err(|e| Error::Msg(format!("resolve cache json: {e}")))?;
        Ok(Some(parsed))
    }

    fn put<T: Serialize>(
        &self,
        table: TableDefinition<&str, &str>,
        key: &str,
        value: &T,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(value)
            .map_err(|e| Error::Msg(format!("resolve cache json: {e}")))?;
        let txn = self.db.begin_write().map_err(cache_err)?;
        {
            let mut table = txn.open_table(table).map_err(cache_err)?;
            table.insert(key, json.as_str()).map_err(cache_err)?;
        }
        txn.commit().map_err(cache_err)?;
        Ok(())
    }
}

fn init_tables(db: &Database) -> Result<(), Error> {
    let txn = db.begin_write().map_err(cache_err)?;
    txn.open_table(SYMBOL_TABLE).map_err(cache_err)?;
    txn.open_table(ACCESSION_TABLE).map_err(cache_err)?;
    txn.commit().map_err(cache_err)?;
    Ok(())
}

fn cache_err(e: impl std::fmt::Display) -> Error {
    Error::Msg(format!("resolve cache: {e}"))
}

/// `v1|symbol|{species}|{SYMBOL_UPPER}|seq={0|1}`
pub fn symbol_key(species: &str, symbol: &str, include_sequence: bool) -> String {
    format!(
        "v1|symbol|{}|{}|seq={}",
        species.trim(),
        symbol.trim().to_ascii_uppercase(),
        if include_sequence { 1 } else { 0 }
    )
}

/// `v1|accession|{ACCESSION_UPPER}`
pub fn accession_key(accession: &str) -> String {
    format!("v1|accession|{}", accession.trim().to_ascii_uppercase())
}

/// Current on-disk path after env resolution (for docs / startup).
pub fn resolved_path() -> PathBuf {
    std::env::var(REDB_PATH_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REDB_PATH))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::Cds;
    use crate::resolve::{ResolveResponse, RetrieveResponse};

    fn sample_resolve() -> ResolveResponse {
        ResolveResponse {
            symbol: "PCSK9".into(),
            name: "proprotein convertase subtilisin/kexin type 9".into(),
            accession: "NM_174936.4".into(),
            ensembl_transcript: "ENST00000302118".into(),
            species: "homo_sapiens".into(),
            cds: Cds { start: 4, end: 9 },
            sequence: "AAAATGCCCTAA".into(),
            length: 12,
            cached: false,
        }
    }

    #[test]
    fn keys_normalize_case() {
        assert_eq!(
            symbol_key("homo_sapiens", " pcsk9 ", true),
            "v1|symbol|homo_sapiens|PCSK9|seq=1"
        );
        assert_eq!(
            symbol_key("homo_sapiens", "PCSK9", false),
            "v1|symbol|homo_sapiens|PCSK9|seq=0"
        );
        assert_eq!(accession_key(" nm_174936.4 "), "v1|accession|NM_174936.4");
    }

    #[test]
    fn put_get_round_trip_temp_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("resolve-cache.redb");
        let cache = ResolveCache::open(&path).expect("open");

        let key = symbol_key("homo_sapiens", "PCSK9", true);
        cache.put_resolve(&key, &sample_resolve()).expect("put");
        let got: ResolveResponse = cache.get_resolve(&key).expect("get").expect("hit");
        assert_eq!(got.symbol, "PCSK9");
        assert_eq!(got.accession, "NM_174936.4");
        assert_eq!(got.sequence, "AAAATGCCCTAA");
        assert_eq!(got.cds, Cds { start: 4, end: 9 });
        assert!(!got.cached, "stored payload must not flip cached");

        let acc_key = accession_key("NM_174936.4");
        let stored = RetrieveResponse {
            accession: "NM_174936.4".into(),
            header: "NM_174936.4 PCSK9".into(),
            sequence: ">NM_174936.4\nATGC\n".into(),
            length: 4,
            cached: false,
        };
        cache.put_retrieve(&acc_key, &stored).expect("put acc");
        let got: RetrieveResponse = cache.get_retrieve(&acc_key).expect("get").expect("hit");
        assert_eq!(got.accession, "NM_174936.4");
        assert_eq!(got.length, 4);
        assert!(!got.cached);

        assert!(cache
            .get_resolve::<ResolveResponse>("v1|symbol|homo_sapiens|NOPE|seq=1")
            .expect("miss")
            .is_none());
    }

    #[test]
    fn in_memory_round_trip() {
        let cache = ResolveCache::in_memory().expect("mem");
        let key = symbol_key("homo_sapiens", "PCSK9", false);
        let mut body = sample_resolve();
        body.sequence.clear();
        body.length = 0;
        cache.put_resolve(&key, &body).expect("put");
        let got: ResolveResponse = cache.get_resolve(&key).unwrap().unwrap();
        assert_eq!(got.symbol, "PCSK9");
        assert!(got.sequence.is_empty());
    }
}
