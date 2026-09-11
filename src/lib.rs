//! siRNA design engine — Rust port of the local verification prototype.
//!
//! Modules land incrementally: `rules` → `tm` → `design` → `ensembl` → `offtarget` → `api`.

pub mod api;
pub mod design;
pub mod ensembl;
pub mod error;
pub mod offtarget;
pub mod resolve;
pub mod resolve_cache;
pub mod rules;
pub mod tm;

pub use api::{
    cors_layer, ensembl_base_url, listen_addr, router, ApiHealth, ApiVersion, AppState,
    DesignRequest, CORS_ALLOW_ORIGIN_ENV, DEFAULT_CORS_ORIGINS, DEFAULT_LISTEN_ADDR,
    ENSEMBL_URL_ENV, LISTEN_ADDR_ENV,
};
pub use design::{
    default_input, design_from_resolved, design_sirnas, normalize_dna, parse_range, passes_combine,
    skip_offtarget, AlgorithmFlags, Cds, DesignInput, DesignMeta, DesignResult, RuleHits,
    SirnaCandidate, SpecificityInfo, TargetRange, Transcript, COMBINE_ALL_AND, COMBINE_UNION,
    COMBINE_U_OR_RA,
};
pub use ensembl::{
    clean_description, locate_cds, parse_xrefs, resolve_by_symbol, resolve_by_symbol_for,
    EnsemblClient, ResolvedTarget, DEFAULT_SPECIES, ENSEMBL_REST,
};
pub use error::Error;
pub use offtarget::{
    check, db_info, default_base_url, guide_to_dna, health, passes_specificity_filter,
    CheckRequest, CheckResponse, DbInfo, HealthStatus, OfftargetClient, OfftargetHit,
    OfftargetQuery, OfftargetResult, DEFAULT_OFFTARGET_URL, OFFTARGET_URL_ENV, QUERY_BATCH_SIZE,
    SPECIFICITY_HIGH, SPECIFICITY_LOW, SPECIFICITY_MEDIUM,
};
pub use resolve::{
    parse_retrieve_fasta, resolve_symbol, retrieve_accession, NcbiClient, ResolveQuery,
    ResolveResponse, RetrieveQuery, RetrieveResponse, NCBI_EFETCH,
};
pub use resolve_cache::{
    accession_key, symbol_key, ResolveCache, DEFAULT_REDB_PATH, REDB_PATH_ENV,
};
pub use rules::{
    combine_all_and, combine_u_or_ra, combine_union, derive_score, evaluate_rules, gc_percent,
    oligos_from_23mer, pass_amarzguioui, pass_reynolds, pass_ui_tei, passes_contiguous_filters,
    ContigOpts, EnabledRules, Oligos, RulesMap, RULE_AMARZGUIOUI, RULE_REYNOLDS, RULE_UI_TEI,
};
pub use tm::{rna_duplex_tm, seed_tm_pair, SeedTmPair};
