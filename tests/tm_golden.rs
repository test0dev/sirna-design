//! Golden regression: seed Tm fields in testdata/pcsk9/pcsk9.atomic.json.

use serde::Deserialize;
use sirna_design::{rna_duplex_tm, seed_tm_pair};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct GoldenFile {
    cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoldenCase {
    start: u32,
    sense: String,
    antisense: String,
    #[serde(rename = "guideSeed")]
    guide_seed: String,
    #[serde(rename = "passengerSeed")]
    passenger_seed: String,
    #[serde(rename = "guideTm")]
    guide_tm: f64,
    #[serde(rename = "passengerTm")]
    passenger_tm: f64,
    #[serde(rename = "seedTmMax")]
    seed_tm_max: f64,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/pcsk9/pcsk9.atomic.json")
}

#[test]
fn pcsk9_atomic_seed_tm_match_every_case() {
    let raw = fs::read_to_string(golden_path()).expect("read golden json");
    let file: GoldenFile = serde_json::from_str(&raw).expect("parse golden json");
    assert!(!file.cases.is_empty(), "expected golden cases");

    for case in &file.cases {
        let guide = rna_duplex_tm(&case.guide_seed);
        let passenger = rna_duplex_tm(&case.passenger_seed);
        assert_eq!(
            guide, case.guide_tm,
            "start {}: guideTm (seed {})",
            case.start, case.guide_seed
        );
        assert_eq!(
            passenger, case.passenger_tm,
            "start {}: passengerTm (seed {})",
            case.start, case.passenger_seed
        );

        let pair = seed_tm_pair(&case.sense, &case.antisense);
        assert_eq!(
            pair.guide, case.guide_tm,
            "start {}: seed_tm_pair.guide",
            case.start
        );
        assert_eq!(
            pair.passenger, case.passenger_tm,
            "start {}: seed_tm_pair.passenger",
            case.start
        );
        assert_eq!(
            pair.max, case.seed_tm_max,
            "start {}: seed_tm_pair.max / seedTmMax",
            case.start
        );

        // Seeds extracted from duplex must match golden seed strings.
        let guide_seed: String = case.antisense.chars().skip(1).take(7).collect();
        let passenger_seed: String = case.sense.chars().skip(1).take(7).collect();
        assert_eq!(
            guide_seed, case.guide_seed,
            "start {}: guideSeed extraction",
            case.start
        );
        assert_eq!(
            passenger_seed, case.passenger_seed,
            "start {}: passengerSeed extraction",
            case.start
        );
    }
}
