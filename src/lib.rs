//! siRNA design engine — Rust port of the local verification prototype.
//!
//! Modules land incrementally: `rules` → `tm` → `design` → `ensembl` → `offtarget` → `api`.

pub mod design;
pub mod error;
pub mod rules;
pub mod tm;

pub use design::{
    default_input, design_from_resolved, design_sirnas, normalize_dna, parse_range, passes_combine,
    skip_offtarget, AlgorithmFlags, Cds, DesignInput, DesignMeta, DesignResult, RuleHits,
    SirnaCandidate, SpecificityInfo, TargetRange, Transcript, COMBINE_ALL_AND, COMBINE_UNION,
    COMBINE_U_OR_RA,
};
pub use error::Error;
pub use rules::{
    combine_all_and, combine_u_or_ra, combine_union, derive_score, evaluate_rules, gc_percent,
    oligos_from_23mer, pass_amarzguioui, pass_reynolds, pass_ui_tei, passes_contiguous_filters,
    ContigOpts, EnabledRules, Oligos, RulesMap, RULE_AMARZGUIOUI, RULE_REYNOLDS, RULE_UI_TEI,
};
pub use tm::{rna_duplex_tm, seed_tm_pair, SeedTmPair};
