//! Golden regression: every case in testdata/pcsk9/pcsk9.atomic.json.

use serde::Deserialize;
use sirna_design::{
    combine_all_and, combine_u_or_ra, combine_union, evaluate_rules, gc_percent, oligos_from_23mer,
    pass_amarzguioui, pass_reynolds, pass_ui_tei, passes_contiguous_filters, ContigOpts,
    EnabledRules, RULE_AMARZGUIOUI, RULE_REYNOLDS, RULE_UI_TEI,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct GoldenFile {
    cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize)]
struct GoldenCase {
    start: u32,
    target23: String,
    sense: String,
    antisense: String,
    gc21: f64,
    gc19: f64,
    #[serde(rename = "uiTei")]
    ui_tei: bool,
    reynolds: bool,
    amarzguioui: bool,
    rules: BTreeMap<String, bool>,
    combine_union: bool,
    combine_u_or_ra: bool,
    combine_all_and: bool,
    contig_pass_default: bool,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/pcsk9/pcsk9.atomic.json")
}

#[test]
fn pcsk9_atomic_rules_match_every_case() {
    let raw = fs::read_to_string(golden_path()).expect("read golden json");
    let file: GoldenFile = serde_json::from_str(&raw).expect("parse golden json");
    assert!(!file.cases.is_empty(), "expected golden cases");

    let enabled = EnabledRules::all();
    let contig = ContigOpts::default_filters();

    for case in &file.cases {
        let oligos = oligos_from_23mer(&case.target23)
            .unwrap_or_else(|e| panic!("start {}: oligos: {e}", case.start));
        assert_eq!(
            oligos.sense, case.sense,
            "start {}: sense mismatch",
            case.start
        );
        assert_eq!(
            oligos.antisense, case.antisense,
            "start {}: antisense mismatch",
            case.start
        );

        let gc21 = gc_percent(&oligos.sense);
        let sense19: String = oligos.sense.chars().take(19).collect();
        let gc19 = gc_percent(&sense19);
        assert_eq!(gc21, case.gc21, "start {}: gc21", case.start);
        assert_eq!(gc19, case.gc19, "start {}: gc19", case.start);

        let ui = pass_ui_tei(&oligos.sense, &oligos.antisense);
        let ry = pass_reynolds(&oligos.sense);
        let am = pass_amarzguioui(&oligos.sense, &oligos.antisense);
        assert_eq!(ui, case.ui_tei, "start {}: uiTei", case.start);
        assert_eq!(ry, case.reynolds, "start {}: reynolds", case.start);
        assert_eq!(am, case.amarzguioui, "start {}: amarzguioui", case.start);

        let rules = evaluate_rules(&oligos.sense, &oligos.antisense);
        assert_eq!(
            *rules.get(RULE_UI_TEI).unwrap(),
            *case.rules.get(RULE_UI_TEI).unwrap(),
            "start {}: rules Ui-Tei",
            case.start
        );
        assert_eq!(
            *rules.get(RULE_REYNOLDS).unwrap(),
            *case.rules.get(RULE_REYNOLDS).unwrap(),
            "start {}: rules Reynolds",
            case.start
        );
        assert_eq!(
            *rules.get(RULE_AMARZGUIOUI).unwrap(),
            *case.rules.get(RULE_AMARZGUIOUI).unwrap(),
            "start {}: rules Amarzguioui",
            case.start
        );

        assert_eq!(
            combine_union(&rules, enabled),
            case.combine_union,
            "start {}: combine_union",
            case.start
        );
        assert_eq!(
            combine_u_or_ra(&rules, enabled),
            case.combine_u_or_ra,
            "start {}: combine_u_or_ra",
            case.start
        );
        assert_eq!(
            combine_all_and(&rules, enabled),
            case.combine_all_and,
            "start {}: combine_all_and",
            case.start
        );

        assert_eq!(
            passes_contiguous_filters(&oligos.sense, contig),
            case.contig_pass_default,
            "start {}: contig_pass_default",
            case.start
        );
    }
}
