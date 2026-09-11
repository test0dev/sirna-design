//! Algorithmic rule filters for siRNA candidate selection.
//!
//! Faithful port of the Node/Bun prototype helpers (Ui-Tei, Reynolds,
//! Amarzguioui, GC%, contiguous runs, combine modes, deriveScore).

use crate::error::Error;
use std::collections::BTreeMap;

/// Canonical rule display names (match golden fixtures / design JSON).
pub const RULE_UI_TEI: &str = "Ui-Tei";
pub const RULE_REYNOLDS: &str = "Reynolds";
pub const RULE_AMARZGUIOUI: &str = "Amarzguioui";

/// Sense + antisense oligos derived from a 23-mer target window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oligos {
    pub sense: String,
    pub antisense: String,
}

/// Options for contiguous-base run filters on the sense strand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContigOpts {
    pub avoid_gc: bool,
    pub gc_min_len: usize,
    pub avoid_at: bool,
    pub at_min_len: usize,
}

impl ContigOpts {
    /// Defaults matching the PCSK9 golden fixtures / design.default
    /// (`avoidContiguousGC/AT = true`, min length 4).
    pub fn default_filters() -> Self {
        Self {
            avoid_gc: true,
            gc_min_len: 4,
            avoid_at: true,
            at_min_len: 4,
        }
    }
}

/// Which algorithms participate in a combine expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnabledRules {
    pub ui_tei: bool,
    pub reynolds: bool,
    pub amarzguioui: bool,
}

impl EnabledRules {
    pub fn all() -> Self {
        Self {
            ui_tei: true,
            reynolds: true,
            amarzguioui: true,
        }
    }
}

/// Per-rule pass/fail results keyed by display name.
pub type RulesMap = BTreeMap<&'static str, bool>;

fn to_rna(seq: &str) -> String {
    seq.to_uppercase().replace('T', "U")
}

fn count_au(seq: &str) -> usize {
    seq.chars().filter(|c| matches!(c, 'A' | 'U')).count()
}

fn has_run(seq: &str, bases: &str, min_len: usize) -> bool {
    if min_len == 0 {
        return true;
    }
    let upper = seq.to_uppercase();
    let bases_upper = bases.to_uppercase();
    let bytes = upper.as_bytes();
    let base_bytes = bases_upper.as_bytes();
    let mut run = 0usize;
    for &b in bytes {
        if base_bytes.contains(&b) {
            run += 1;
            if run >= min_len {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// GC% with JS `Number#toFixed(1)` rounding (one decimal place).
pub fn gc_percent(rna: &str) -> f64 {
    if rna.is_empty() {
        return 0.0;
    }
    let gc = rna.chars().filter(|c| matches!(c, 'G' | 'C' | 'g' | 'c')).count();
    let pct = (gc as f64 / rna.len() as f64) * 100.0;
    // Match JS toFixed(1): round half away from zero, then one decimal.
    js_to_fixed_1(pct)
}

fn js_to_fixed_1(x: f64) -> f64 {
    let scaled = x * 10.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    rounded / 10.0
}

/// Derive 21-nt sense / antisense oligos from a 23-mer DNA/RNA target window.
pub fn oligos_from_23mer(target23: &str) -> Result<Oligos, Error> {
    let dna = target23.to_uppercase().replace('U', "T");
    if dna.len() != 23 {
        return Err(Error::Msg(format!(
            "expected 23-mer, got {}",
            dna.len()
        )));
    }
    let sense = to_rna(&dna[2..23]);
    let map = |b: u8| -> char {
        match b {
            b'A' => 'U',
            b'T' => 'A',
            b'G' => 'C',
            b'C' => 'G',
            _ => 'N',
        }
    };
    let antisense: String = dna[..21]
        .bytes()
        .rev()
        .map(map)
        .collect();
    Ok(Oligos { sense, antisense })
}

/// Ui-Tei asymmetry / composition filter.
pub fn pass_ui_tei(sense: &str, antisense: &str) -> bool {
    let guide5 = match antisense.chars().next() {
        Some(c) => c,
        None => return false,
    };
    let pass5 = match sense.chars().next() {
        Some(c) => c,
        None => return false,
    };
    if !"AU".contains(guide5) {
        return false;
    }
    if !"GC".contains(pass5) {
        return false;
    }
    let guide_prefix: String = antisense.chars().take(7).collect();
    if count_au(&guide_prefix) < 4 {
        return false;
    }
    let sense19: String = sense.chars().take(19).collect();
    if has_run(&sense19, "GC", 9) {
        return false;
    }
    true
}

/// Reynolds scoring rule (threshold ≥ 6 on first 19 nt of sense).
pub fn pass_reynolds(sense: &str) -> bool {
    let s: String = sense.chars().take(19).collect();
    if s.len() < 19 {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    let gc_count = chars.iter().filter(|c| matches!(c, 'G' | 'C')).count();
    let mut score: i32 = 0;
    if (7..=10).contains(&gc_count) {
        score += 1;
    }
    let tail: String = chars[14..19].iter().collect();
    score += count_au(&tail) as i32;
    if chars[18] == 'A' {
        score += 1;
    }
    if chars[2] == 'A' {
        score += 1;
    }
    if chars[9] == 'U' {
        score += 1;
    }
    if "GC".contains(chars[18]) {
        score -= 1;
    }
    if chars[12] == 'G' {
        score -= 1;
    }
    score >= 6
}

/// Amarzguioui differential AU / position scoring (threshold ≥ 3).
pub fn pass_amarzguioui(sense: &str, antisense: &str) -> bool {
    let s: String = sense.chars().take(19).collect();
    let a: String = antisense.chars().take(19).collect();
    if s.len() < 19 || a.len() < 19 {
        return false;
    }
    let s_chars: Vec<char> = s.chars().collect();
    let a_chars: Vec<char> = a.chars().collect();
    let a_head: String = a_chars[..3].iter().collect();
    let s_head: String = s_chars[..3].iter().collect();
    let asym3 = count_au(&a_head) as i32 - count_au(&s_head) as i32;
    let s1 = "GC".contains(s_chars[0]);
    let a6 = s_chars[5] == 'A';
    let w19 = "AU".contains(s_chars[18]);
    let u1 = s_chars[0] == 'U';
    let g19 = s_chars[18] == 'G';
    let score = asym3
        + (if s1 { 1 } else { 0 })
        + (if a6 { 1 } else { 0 })
        + (if w19 { 1 } else { 0 })
        + (if u1 { -1 } else { 0 })
        + (if g19 { -1 } else { 0 });
    score >= 3
}

/// Evaluate all three named rules into a map matching golden `rules` objects.
pub fn evaluate_rules(sense: &str, antisense: &str) -> RulesMap {
    let mut m = BTreeMap::new();
    m.insert(RULE_UI_TEI, pass_ui_tei(sense, antisense));
    m.insert(RULE_REYNOLDS, pass_reynolds(sense));
    m.insert(RULE_AMARZGUIOUI, pass_amarzguioui(sense, antisense));
    m
}

/// Effective pass bit: disabled → vacuous `true` (AND / union).
fn eff_vacuous(rules: &RulesMap, name: &str, enabled: bool) -> bool {
    if !enabled {
        return true;
    }
    *rules.get(name).unwrap_or(&false)
}

/// Effective pass bit: disabled → `false` (used by U || (R && A) operands).
fn eff_false(rules: &RulesMap, name: &str, enabled: bool) -> bool {
    if !enabled {
        return false;
    }
    *rules.get(name).unwrap_or(&false)
}

/// `"Ui-Tei × Reynolds × Amarzguioui"` — AND among enabled (disabled vacuous true).
pub fn combine_all_and(rules: &RulesMap, enabled: EnabledRules) -> bool {
    eff_vacuous(rules, RULE_UI_TEI, enabled.ui_tei)
        && eff_vacuous(rules, RULE_REYNOLDS, enabled.reynolds)
        && eff_vacuous(rules, RULE_AMARZGUIOUI, enabled.amarzguioui)
}

/// `"Ui-Tei + Reynolds × Amarzguioui"` — `U || (R && A)` with disabled → false.
pub fn combine_u_or_ra(rules: &RulesMap, enabled: EnabledRules) -> bool {
    let u = eff_false(rules, RULE_UI_TEI, enabled.ui_tei);
    let r = eff_false(rules, RULE_REYNOLDS, enabled.reynolds);
    let a = eff_false(rules, RULE_AMARZGUIOUI, enabled.amarzguioui);
    u || (r && a)
}

/// `"Ui-Tei + Reynolds + Amarzguioui"` — union among enabled; none enabled → true.
pub fn combine_union(rules: &RulesMap, enabled: EnabledRules) -> bool {
    let any_enabled = enabled.ui_tei || enabled.reynolds || enabled.amarzguioui;
    if !any_enabled {
        return true;
    }
    let mut any = false;
    if enabled.ui_tei {
        any |= *rules.get(RULE_UI_TEI).unwrap_or(&false);
    }
    if enabled.reynolds {
        any |= *rules.get(RULE_REYNOLDS).unwrap_or(&false);
    }
    if enabled.amarzguioui {
        any |= *rules.get(RULE_AMARZGUIOUI).unwrap_or(&false);
    }
    any
}

/// Contiguous GC / AU run filters on sense (RNA uses AU for “AT”).
pub fn passes_contiguous_filters(sense: &str, opts: ContigOpts) -> bool {
    if opts.avoid_gc && has_run(sense, "GC", opts.gc_min_len) {
        return false;
    }
    if opts.avoid_at && has_run(sense, "AU", opts.at_min_len) {
        return false;
    }
    true
}

/// Composite score from rule pass count + seed Tm term.
pub fn derive_score(rules: &RulesMap, seed_tm: f64) -> f64 {
    let passed = rules.values().filter(|&&v| v).count() as f64;
    let tm_score = (1.0 - seed_tm / 30.0).max(0.0);
    js_to_fixed_2(0.7 * (passed / 3.0) + 0.3 * tm_score)
}

fn js_to_fixed_2(x: f64) -> f64 {
    let scaled = x * 100.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    rounded / 100.0
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn gc_percent_empty_and_rounding() {
        assert_eq!(gc_percent(""), 0.0);
        assert_eq!(gc_percent("GCGCGCGCGCGCGCGCGCGCG"), 100.0);
        assert_eq!(gc_percent("AAAAAAAAAAAAAAAAAAAAA"), 0.0);
    }

    #[test]
    fn oligos_length_error() {
        assert!(oligos_from_23mer("ATGC").is_err());
    }

    #[test]
    fn contig_default_matches_shape() {
        let opts = ContigOpts::default_filters();
        assert!(opts.avoid_gc && opts.avoid_at);
        assert_eq!(opts.gc_min_len, 4);
        assert_eq!(opts.at_min_len, 4);
    }
}
