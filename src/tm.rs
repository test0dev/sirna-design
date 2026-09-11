//! siDirect-style RNA–RNA nearest-neighbor seed Tm.
//!
//! Faithful port of the Node/Bun `rnaDuplexTm` / `seedTmPair` helpers.

const HELIX_INIT_A: f64 = -10.8;
const R_GAS: f64 = 1.987;
const CT_MOLAR: f64 = 100e-6;
const NA_MOLAR: f64 = 0.1;

fn to_rna(seq: &str) -> String {
    seq.to_uppercase().replace('T', "U")
}

fn rna_dh(dinuc: &str) -> Option<f64> {
    Some(match dinuc {
        "AA" | "UU" => -6.6,
        "AU" => -5.7,
        "UA" => -8.1,
        "CA" | "UG" => -10.5,
        "CU" | "AG" => -7.6,
        "GA" | "UC" => -13.3,
        "GU" | "AC" => -10.2,
        "CG" => -8.0,
        "GC" => -14.2,
        "GG" | "CC" => -12.2,
        _ => return None,
    })
}

fn rna_ds(dinuc: &str) -> Option<f64> {
    Some(match dinuc {
        "AA" | "UU" => -18.4,
        "AU" => -15.5,
        "UA" => -22.6,
        "CA" | "UG" => -27.8,
        "CU" | "AG" => -19.2,
        "GA" | "UC" => -35.5,
        "GU" | "AC" => -26.2,
        "CG" => -19.4,
        "GC" => -34.9,
        "GG" | "CC" => -29.7,
        _ => return None,
    })
}

/// Match JS `Number(x.toFixed(1))`.
fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// RNA duplex melting temperature (°C) via nearest-neighbor parameters.
///
/// Returns `0` when the sequence length is less than 2 (after T→U).
/// Panics if a dinucleotide is missing from the NN tables.
pub fn rna_duplex_tm(seq: &str) -> f64 {
    let s = to_rna(seq);
    if s.len() < 2 {
        return 0.0;
    }

    let mut d_h = 0.0_f64;
    let mut d_s = 0.0_f64;
    let bytes = s.as_bytes();
    for i in 0..bytes.len() - 1 {
        let dinuc = std::str::from_utf8(&bytes[i..i + 2]).expect("RNA is ASCII");
        let h = rna_dh(dinuc).unwrap_or_else(|| panic!("missing NN for {dinuc}"));
        let ent = rna_ds(dinuc).unwrap_or_else(|| panic!("missing NN for {dinuc}"));
        d_h += h;
        d_s += ent;
    }

    // JS: Math.log = ln, Math.log10 = log10
    let tm = (1000.0 * d_h) / (HELIX_INIT_A + d_s + R_GAS * (CT_MOLAR / 4.0).ln()) - 273.15
        + 16.6 * NA_MOLAR.log10();
    round1(tm)
}

/// Guide / passenger seed Tm pair for a sense + antisense duplex.
///
/// Guide seed = antisense positions 2–8 (0-based slice `[1..8]`);
/// passenger seed = sense positions 2–8.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeedTmPair {
    pub guide: f64,
    pub passenger: f64,
    pub max: f64,
}

/// Compute guide, passenger, and max seed Tm for an siRNA duplex.
pub fn seed_tm_pair(sense: &str, antisense: &str) -> SeedTmPair {
    let guide = rna_duplex_tm(seed_slice(antisense));
    let passenger = rna_duplex_tm(seed_slice(sense));
    SeedTmPair {
        guide,
        passenger,
        max: guide.max(passenger),
    }
}

/// Positions 2–8 (1-based) → byte range `[1..8]` for ASCII RNA/DNA.
fn seed_slice(seq: &str) -> &str {
    let end = seq.len().min(8);
    if end < 2 {
        return "";
    }
    &seq[1..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rna_duplex_tm_len_lt_2_is_zero() {
        assert_eq!(rna_duplex_tm(""), 0.0);
        assert_eq!(rna_duplex_tm("A"), 0.0);
        assert_eq!(rna_duplex_tm("a"), 0.0);
        assert_eq!(rna_duplex_tm("T"), 0.0);
        assert_eq!(rna_duplex_tm("U"), 0.0);
    }

    #[test]
    fn rna_duplex_tm_accepts_dna_t_as_u() {
        // CUGGAGC from golden case start 291 → 40.3
        assert_eq!(rna_duplex_tm("CUGGAGC"), 40.3);
        assert_eq!(rna_duplex_tm("CTGGAGC"), 40.3);
    }

    #[test]
    fn seed_tm_pair_matches_known_pcsk9_case() {
        // start 291
        let pair = seed_tm_pair("GGGCACCGUCAGCUCCAGGCG", "CCUGGAGCUGACGGUGCCCAU");
        assert_eq!(pair.guide, 40.3);
        assert_eq!(pair.passenger, 47.5);
        assert_eq!(pair.max, 47.5);
    }
}
