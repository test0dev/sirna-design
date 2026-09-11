//! Design pipeline: slide a 23-mer window, filter, score, emit candidates.
//!
//! Faithful port of the Node/Bun `defaultInput` / `designSirnas` /
//! `designFromResolved` helpers. Ensembl resolve and remote offtarget are
//! intentionally out of scope here: when `specificity` is `"none"` or no
//! offtarget index is supplied, offtarget filtering is skipped.

use crate::error::Error;
use crate::rules::{
    combine_all_and, combine_u_or_ra, combine_union, derive_score, evaluate_rules, gc_percent,
    oligos_from_23mer, passes_contiguous_filters, ContigOpts, EnabledRules, RulesMap,
    RULE_AMARZGUIOUI, RULE_REYNOLDS, RULE_UI_TEI,
};
use crate::tm::seed_tm_pair;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Default combine expression (union of the three named algorithms).
pub const COMBINE_UNION: &str = "Ui-Tei + Reynolds + Amarzguioui";
/// `U || (R && A)`.
pub const COMBINE_U_OR_RA: &str = "Ui-Tei + Reynolds \u{00d7} Amarzguioui";
/// AND of the three named algorithms.
pub const COMBINE_ALL_AND: &str = "Ui-Tei \u{00d7} Reynolds \u{00d7} Amarzguioui";

const SIRNA_LEN: u32 = 21;
const OVERHANG: u32 = 2;
const WINDOW: usize = 23;

fn default_true() -> bool {
    true
}

fn default_seed_tm_max() -> f64 {
    21.5
}

fn default_specificity() -> String {
    "human-230".into()
}

fn default_contig_min() -> usize {
    4
}

fn default_gc_min() -> f64 {
    30.0
}

fn default_gc_max() -> f64 {
    52.0
}

fn default_combine() -> String {
    COMBINE_UNION.into()
}

/// Per-algorithm enable flags (serde names match golden / UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlgorithmFlags {
    #[serde(rename = "Ui-Tei", default = "default_true")]
    pub ui_tei: bool,
    #[serde(rename = "Reynolds", default = "default_true")]
    pub reynolds: bool,
    #[serde(rename = "Amarzguioui", default = "default_true")]
    pub amarzguioui: bool,
}

impl AlgorithmFlags {
    pub fn all() -> Self {
        Self {
            ui_tei: true,
            reynolds: true,
            amarzguioui: true,
        }
    }

    pub fn to_enabled(self) -> EnabledRules {
        EnabledRules {
            ui_tei: self.ui_tei,
            reynolds: self.reynolds,
            amarzguioui: self.amarzguioui,
        }
    }

    /// Enabled algorithm names in canonical display order.
    pub fn enabled_names(self) -> Vec<String> {
        let mut names = Vec::new();
        if self.ui_tei {
            names.push(RULE_UI_TEI.to_string());
        }
        if self.reynolds {
            names.push(RULE_REYNOLDS.to_string());
        }
        if self.amarzguioui {
            names.push(RULE_AMARZGUIOUI.to_string());
        }
        names
    }
}

impl Default for AlgorithmFlags {
    fn default() -> Self {
        Self::all()
    }
}

/// User-facing design request (camelCase, matches `pcsk9.meta.json` `input`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignInput {
    #[serde(default)]
    pub gene_symbol: String,
    #[serde(default)]
    pub accession: String,
    #[serde(default)]
    pub sequence: String,
    #[serde(default)]
    pub algorithms: AlgorithmFlags,
    #[serde(default = "default_combine")]
    pub combine: String,
    #[serde(default = "default_seed_tm_max", deserialize_with = "de_f64_like")]
    pub seed_tm_max: f64,
    #[serde(default = "default_specificity")]
    pub specificity: String,
    #[serde(default = "default_true")]
    pub hide_less_specific: bool,
    #[serde(default = "default_true")]
    pub show_off_target_hits: bool,
    #[serde(default)]
    pub target_range_from: String,
    #[serde(default)]
    pub target_range_to: String,
    #[serde(default = "default_true", rename = "avoidContiguousGC")]
    pub avoid_contiguous_gc: bool,
    #[serde(
        default = "default_contig_min",
        deserialize_with = "de_usize_like",
        rename = "avoidContiguousGCMin"
    )]
    pub avoid_contiguous_gc_min: usize,
    #[serde(default = "default_true", rename = "avoidContiguousAT")]
    pub avoid_contiguous_at: bool,
    #[serde(
        default = "default_contig_min",
        deserialize_with = "de_usize_like",
        rename = "avoidContiguousATMin"
    )]
    pub avoid_contiguous_at_min: usize,
    #[serde(default = "default_gc_min", deserialize_with = "de_f64_like")]
    pub gc_min: f64,
    #[serde(default = "default_gc_max", deserialize_with = "de_f64_like")]
    pub gc_max: f64,
    #[serde(default = "default_true")]
    pub match_all_criteria: bool,
}

impl Default for DesignInput {
    fn default() -> Self {
        default_input()
    }
}

/// Defaults matching the Node `defaultInput()` helper.
pub fn default_input() -> DesignInput {
    DesignInput {
        gene_symbol: String::new(),
        accession: String::new(),
        sequence: String::new(),
        algorithms: AlgorithmFlags::all(),
        combine: COMBINE_UNION.to_string(),
        seed_tm_max: 21.5,
        specificity: "human-230".into(),
        hide_less_specific: true,
        show_off_target_hits: true,
        target_range_from: String::new(),
        target_range_to: String::new(),
        avoid_contiguous_gc: true,
        avoid_contiguous_gc_min: 4,
        avoid_contiguous_at: true,
        avoid_contiguous_at_min: 4,
        gc_min: 30.0,
        gc_max: 52.0,
        match_all_criteria: true,
    }
}

/// Resolved transcript record (golden `transcript` object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transcript {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub length: u32,
    pub sequence: String,
}

/// 1-based inclusive CDS / target coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cds {
    pub start: u32,
    pub end: u32,
}

/// Inclusive 1-based window used for sliding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRange {
    pub from: u32,
    pub to: u32,
}

/// Specificity block written into design output (no remote lookup here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificityInfo {
    pub species: String,
    pub database: String,
    pub hide_less_specific: bool,
    pub show_off_target_hits: bool,
}

/// Design parameters echoed in the result (golden `design` object).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignMeta {
    pub length: u32,
    pub overhang: u32,
    pub algorithms: Vec<String>,
    pub combine: String,
    pub specificity: SpecificityInfo,
    pub seed_tm_max: f64,
    pub gc_min: f64,
    pub gc_max: f64,
    #[serde(rename = "avoidContiguousGC")]
    pub avoid_contiguous_gc: u32,
    #[serde(rename = "avoidContiguousAT")]
    pub avoid_contiguous_at: u32,
    pub target_range: TargetRange,
}

/// Per-candidate rule pass/fail, keyed by display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleHits {
    #[serde(rename = "Ui-Tei")]
    pub ui_tei: bool,
    #[serde(rename = "Reynolds")]
    pub reynolds: bool,
    #[serde(rename = "Amarzguioui")]
    pub amarzguioui: bool,
}

impl RuleHits {
    fn from_map(rules: &RulesMap) -> Self {
        Self {
            ui_tei: *rules.get(RULE_UI_TEI).unwrap_or(&false),
            reynolds: *rules.get(RULE_REYNOLDS).unwrap_or(&false),
            amarzguioui: *rules.get(RULE_AMARZGUIOUI).unwrap_or(&false),
        }
    }
}

/// One emitted siRNA candidate (golden `sirnas[]` item).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SirnaCandidate {
    pub id: String,
    pub start: u32,
    pub end: u32,
    pub sense: String,
    pub antisense: String,
    pub gc: f64,
    pub seed_tm: f64,
    pub score: f64,
    pub rules: RuleHits,
}

/// Full design result matching the PCSK9 golden JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignResult {
    pub transcript: Transcript,
    pub cds: Cds,
    pub design: DesignMeta,
    pub sirnas: Vec<SirnaCandidate>,
}

/// Uppercase, U→T, drop anything that is not ACGT.
pub fn normalize_dna(sequence: &str) -> String {
    sequence
        .chars()
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u == 'U' {
                'T'
            } else {
                u
            }
        })
        .filter(|c| matches!(c, 'A' | 'C' | 'G' | 'T'))
        .collect()
}

fn parse_pos(raw: &str) -> Option<usize> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let n: usize = s.parse().ok()?;
    (n >= 1).then_some(n)
}

/// Parse a 1-based inclusive target window.
///
/// Empty `from` / `to` fall back to `cds`; missing CDS falls back to `[1, len]`.
/// Results are clamped to `[1, len]` and swapped if inverted.
pub fn parse_range(from: &str, to: &str, cds: Option<Cds>, len: usize) -> TargetRange {
    if len == 0 {
        return TargetRange { from: 1, to: 1 };
    }
    let mut a = parse_pos(from)
        .or_else(|| cds.map(|c| c.start as usize))
        .unwrap_or(1);
    let mut b = parse_pos(to)
        .or_else(|| cds.map(|c| c.end as usize))
        .unwrap_or(len);
    a = a.clamp(1, len);
    b = b.clamp(1, len);
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    TargetRange {
        from: a as u32,
        to: b as u32,
    }
}

fn count_times(combine: &str) -> usize {
    combine
        .chars()
        .filter(|&c| matches!(c, '\u{00d7}' | 'x' | 'X' | '*'))
        .count()
}

/// Evaluate a combine expression against a rule map.
///
/// - two `×` (or `x`/`*`) → AND
/// - one `×` → `Ui-Tei || (Reynolds && Amarzguioui)`
/// - otherwise → union
pub fn passes_combine(rules: &RulesMap, combine: &str, enabled: EnabledRules) -> bool {
    match count_times(combine) {
        n if n >= 2 => combine_all_and(rules, enabled),
        1 => combine_u_or_ra(rules, enabled),
        _ => combine_union(rules, enabled),
    }
}

fn species_label(specificity: &str) -> String {
    let s = specificity.to_ascii_lowercase();
    if s.starts_with("mouse") {
        "Mouse (Mus musculus)".into()
    } else {
        "Human (Homo sapiens)".into()
    }
}

fn specificity_database(specificity: &str) -> String {
    let s = specificity.trim();
    if s.is_empty() {
        "none".into()
    } else {
        s.to_string()
    }
}

/// True when offtarget filtering must be skipped (this PR never supplies an index).
pub fn skip_offtarget(specificity: &str, offtarget_index_supplied: bool) -> bool {
    let s = specificity.trim();
    s.is_empty() || s.eq_ignore_ascii_case("none") || !offtarget_index_supplied
}

fn contig_report(avoid: bool, min_len: usize) -> u32 {
    if avoid {
        min_len as u32
    } else {
        0
    }
}

fn build_design_meta(input: &DesignInput, range: TargetRange) -> DesignMeta {
    DesignMeta {
        length: SIRNA_LEN,
        overhang: OVERHANG,
        algorithms: input.algorithms.enabled_names(),
        combine: input.combine.clone(),
        specificity: SpecificityInfo {
            species: species_label(&input.specificity),
            database: specificity_database(&input.specificity),
            hide_less_specific: input.hide_less_specific,
            show_off_target_hits: input.show_off_target_hits,
        },
        seed_tm_max: input.seed_tm_max,
        gc_min: input.gc_min,
        gc_max: input.gc_max,
        avoid_contiguous_gc: contig_report(
            input.avoid_contiguous_gc,
            input.avoid_contiguous_gc_min,
        ),
        avoid_contiguous_at: contig_report(
            input.avoid_contiguous_at,
            input.avoid_contiguous_at_min,
        ),
        target_range: range,
    }
}

fn is_acgt(seq: &str) -> bool {
    !seq.is_empty() && seq.bytes().all(|b| matches!(b, b'A' | b'C' | b'G' | b'T'))
}

fn scan_candidates(input: &DesignInput, dna: &str, range: TargetRange) -> Vec<SirnaCandidate> {
    let len = dna.len();
    let from = range.from as usize;
    let to = range.to as usize;
    if len < WINDOW || to < WINDOW - 1 || from == 0 {
        return Vec::new();
    }
    let last_start = to - (WINDOW - 1);
    if last_start < from {
        return Vec::new();
    }

    let contig = ContigOpts {
        avoid_gc: input.avoid_contiguous_gc,
        gc_min_len: input.avoid_contiguous_gc_min,
        avoid_at: input.avoid_contiguous_at,
        at_min_len: input.avoid_contiguous_at_min,
    };
    let enabled = input.algorithms.to_enabled();
    // Offtarget filtering is skipped: this pipeline never takes an index, and
    // the PCSK9 goldens set specificity=none.

    let mut sirnas = Vec::new();
    for start in from..=last_start {
        let idx = start - 1;
        let Some(target23) = dna.get(idx..idx + WINDOW) else {
            continue;
        };
        if target23.len() != WINDOW || !is_acgt(target23) {
            continue;
        }
        let oligos = match oligos_from_23mer(target23) {
            Ok(o) => o,
            Err(_) => continue,
        };
        let sense19: String = oligos.sense.chars().take(19).collect();
        let gc_duplex = gc_percent(&sense19);
        let gc = gc_percent(&oligos.sense);
        if gc_duplex < input.gc_min || gc_duplex > input.gc_max {
            continue;
        }
        if !passes_contiguous_filters(&oligos.sense, contig) {
            continue;
        }
        let tm = seed_tm_pair(&oligos.sense, &oligos.antisense);
        if tm.max > input.seed_tm_max {
            continue;
        }
        let rules = evaluate_rules(&oligos.sense, &oligos.antisense);
        if !passes_combine(&rules, &input.combine, enabled) {
            continue;
        }
        let n = sirnas.len() + 1;
        sirnas.push(SirnaCandidate {
            id: format!("si-{n:02}"),
            start: start as u32,
            end: (start + WINDOW - 1) as u32,
            sense: oligos.sense,
            antisense: oligos.antisense,
            gc,
            seed_tm: tm.max,
            score: derive_score(&rules, tm.max),
            rules: RuleHits::from_map(&rules),
        });
    }
    sirnas
}

/// Design from an already-resolved transcript + CDS.
///
/// Offtarget filtering is skipped when `specificity` is `"none"` or when no
/// offtarget index is supplied (this function never takes one).
pub fn design_from_resolved(
    input: &DesignInput,
    transcript: &Transcript,
    cds: Cds,
) -> Result<DesignResult, Error> {
    let dna = normalize_dna(&transcript.sequence);
    if dna.is_empty() {
        return Err(Error::Msg(
            "transcript sequence is empty after ACGT normalize".into(),
        ));
    }
    let range = parse_range(
        &input.target_range_from,
        &input.target_range_to,
        Some(cds),
        dna.len(),
    );
    let sirnas = scan_candidates(input, &dna, range);
    Ok(DesignResult {
        transcript: transcript.clone(),
        cds,
        design: build_design_meta(input, range),
        sirnas,
    })
}

/// Design from a sequence-bearing input.
///
/// Does **not** call Ensembl. A non-empty `sequence` is required.
pub fn design_sirnas(input: &DesignInput) -> Result<DesignResult, Error> {
    let dna = normalize_dna(&input.sequence);
    if dna.is_empty() {
        return Err(Error::Msg(
            "sequence is required (Ensembl resolve is not available in this module)".into(),
        ));
    }
    let len = dna.len();
    let range = parse_range(&input.target_range_from, &input.target_range_to, None, len);
    let transcript = Transcript {
        id: if !input.accession.is_empty() {
            input.accession.clone()
        } else {
            input.gene_symbol.clone()
        },
        symbol: input.gene_symbol.clone(),
        name: input.gene_symbol.clone(),
        length: len as u32,
        sequence: dna,
    };
    let cds = Cds {
        start: range.from,
        end: range.to,
    };
    design_from_resolved(input, &transcript, cds)
}

// --- flexible number deserializers (JSON number or numeric string) ---

fn de_f64_like<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    struct F64Like;
    impl<'de> Visitor<'de> for F64Like {
        type Value = f64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a number or numeric string")
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.trim().parse().map_err(E::custom)
        }
    }
    deserializer.deserialize_any(F64Like)
}

fn de_usize_like<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    struct UsizeLike;
    impl<'de> Visitor<'de> for UsizeLike {
        type Value = usize;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a non-negative integer or numeric string")
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<usize, E> {
            Ok(v as usize)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<usize, E> {
            if v < 0 {
                return Err(E::custom("expected non-negative integer"));
            }
            Ok(v as usize)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<usize, E> {
            if v < 0.0 || !v.is_finite() {
                return Err(E::custom("expected non-negative integer"));
            }
            Ok(v as usize)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<usize, E> {
            v.trim().parse().map_err(E::custom)
        }
    }
    deserializer.deserialize_any(UsizeLike)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn default_input_matches_spec() {
        let d = default_input();
        assert!(d.algorithms.ui_tei && d.algorithms.reynolds && d.algorithms.amarzguioui);
        assert_eq!(d.combine, COMBINE_UNION);
        assert_eq!(d.seed_tm_max, 21.5);
        assert_eq!(d.specificity, "human-230");
        assert!(d.hide_less_specific && d.show_off_target_hits);
        assert!(d.avoid_contiguous_gc && d.avoid_contiguous_at);
        assert_eq!(d.avoid_contiguous_gc_min, 4);
        assert_eq!(d.avoid_contiguous_at_min, 4);
        assert_eq!(d.gc_min, 30.0);
        assert_eq!(d.gc_max, 52.0);
    }

    #[test]
    fn normalize_dna_upper_u_to_t_strips() {
        assert_eq!(normalize_dna(" acguN\nX"), "ACGT");
        assert_eq!(normalize_dna("ttUu"), "TTTT");
    }

    #[test]
    fn parse_range_empty_falls_back_to_cds_then_len() {
        let cds = Cds {
            start: 291,
            end: 2369,
        };
        let r = parse_range("", "", Some(cds), 3637);
        assert_eq!(
            r,
            TargetRange {
                from: 291,
                to: 2369
            }
        );
        let r = parse_range("", "", None, 100);
        assert_eq!(r, TargetRange { from: 1, to: 100 });
        let r = parse_range("10", "5", None, 20);
        assert_eq!(r, TargetRange { from: 5, to: 10 });
        let r = parse_range("0", "9999", Some(cds), 50);
        // "0" is invalid → CDS start (then clamp); 9999 clamps to len
        assert_eq!(r, TargetRange { from: 50, to: 50 });
        let r = parse_range("", "", Some(cds), 4000);
        assert_eq!(
            r,
            TargetRange {
                from: 291,
                to: 2369
            }
        );
    }

    #[test]
    fn skip_offtarget_when_none_or_no_index() {
        assert!(skip_offtarget("none", true));
        assert!(skip_offtarget("none", false));
        assert!(skip_offtarget("human-230", false));
        assert!(!skip_offtarget("human-230", true));
        assert!(skip_offtarget("", true));
    }

    #[test]
    fn design_sirnas_requires_sequence() {
        let err = design_sirnas(&default_input()).unwrap_err();
        assert!(err.to_string().contains("sequence is required"));
    }

    #[test]
    fn passes_combine_dispatches_on_times_vs_plus() {
        let mut rules = RulesMap::new();
        rules.insert(RULE_UI_TEI, true);
        rules.insert(RULE_REYNOLDS, false);
        rules.insert(RULE_AMARZGUIOUI, false);
        let en = EnabledRules::all();
        assert!(passes_combine(&rules, COMBINE_UNION, en));
        assert!(passes_combine(&rules, COMBINE_U_OR_RA, en));
        assert!(!passes_combine(&rules, COMBINE_ALL_AND, en));
    }
}
