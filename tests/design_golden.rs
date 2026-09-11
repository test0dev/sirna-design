//! Golden regression: PCSK9 design pipeline vs default / all-AND fixtures.

use pretty_assertions::assert_eq;
use serde::Deserialize;
use sirna_design::{
    default_input, design_from_resolved, design_sirnas, Cds, DesignInput, DesignResult, Transcript,
    COMBINE_ALL_AND, COMBINE_UNION,
};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaFile {
    input: DesignInput,
    cds: Cds,
    transcript: Transcript,
}

fn testdata(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/pcsk9")
        .join(name)
}

fn load_meta() -> MetaFile {
    let raw = fs::read_to_string(testdata("pcsk9.meta.json")).expect("read meta");
    serde_json::from_str(&raw).expect("parse meta")
}

fn load_design(name: &str) -> DesignResult {
    let raw = fs::read_to_string(testdata(name)).expect("read design golden");
    serde_json::from_str(&raw).expect("parse design golden")
}

fn assert_candidates_match(got: &DesignResult, exp: &DesignResult, label: &str) {
    assert_eq!(
        got.sirnas.len(),
        exp.sirnas.len(),
        "{label}: candidate count"
    );
    for (i, (g, e)) in got.sirnas.iter().zip(exp.sirnas.iter()).enumerate() {
        assert_eq!(g.id, e.id, "{label}[{i}]: id");
        assert_eq!(g.start, e.start, "{label}[{i}]: start");
        assert_eq!(g.end, e.end, "{label}[{i}]: end");
        assert_eq!(g.sense, e.sense, "{label}[{i}]: sense");
        assert_eq!(g.antisense, e.antisense, "{label}[{i}]: antisense");
        assert_eq!(g.gc, e.gc, "{label}[{i}]: gc");
        assert_eq!(g.seed_tm, e.seed_tm, "{label}[{i}]: seedTm");
        assert_eq!(g.score, e.score, "{label}[{i}]: score");
        assert_eq!(g.rules, e.rules, "{label}[{i}]: rules");
    }
}

#[test]
fn default_input_serde_overlay_keeps_spec_defaults() {
    let d = default_input();
    assert_eq!(d.combine, COMBINE_UNION);
    assert_eq!(d.specificity, "human-230");
    assert!(d.hide_less_specific);
    assert!(d.show_off_target_hits);
    assert_eq!(d.seed_tm_max, 21.5);
    assert_eq!(d.gc_min, 30.0);
    assert_eq!(d.gc_max, 52.0);
}

#[test]
fn pcsk9_design_default_combine_matches_golden() {
    let meta = load_meta();
    assert_eq!(meta.input.combine, COMBINE_UNION);
    assert_eq!(meta.input.specificity, "none");

    let got = design_from_resolved(&meta.input, &meta.transcript, meta.cds)
        .expect("design_from_resolved default");
    let exp = load_design("pcsk9.design.default.json");

    assert_eq!(got.transcript, exp.transcript);
    assert_eq!(got.cds, exp.cds);
    assert_eq!(got.design, exp.design);
    assert_candidates_match(&got, &exp, "default");
    assert_eq!(got, exp);

    // Sequence-only entry point must emit the same candidates (no Ensembl).
    let via_seq = design_sirnas(&meta.input).expect("design_sirnas");
    assert_candidates_match(&via_seq, &got, "design_sirnas vs from_resolved");
}

#[test]
fn pcsk9_design_all_and_combine_matches_golden() {
    let meta = load_meta();
    let mut input = meta.input;
    input.combine = COMBINE_ALL_AND.to_string();

    let got = design_from_resolved(&input, &meta.transcript, meta.cds)
        .expect("design_from_resolved all-and");
    let exp = load_design("pcsk9.design.alland.json");

    assert_eq!(got.design.combine, COMBINE_ALL_AND);
    assert_eq!(got.transcript, exp.transcript);
    assert_eq!(got.cds, exp.cds);
    assert_eq!(got.design, exp.design);
    assert_candidates_match(&got, &exp, "alland");
    assert_eq!(got, exp);
}
