//! R5: each rule's `cases.toml` gives the interpreter's findings and the fused evaluator's, the same rows rule by rule in the same
//! rule order, on the rule's own cases and on the engine's Python fixtures.

use crate::fixtures::{fused, interpreted, ruff, Emission};
use crate::symbols::engine_fixtures;

#[derive(serde::Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    files: std::collections::BTreeMap<String, String>,
    expected: Vec<Expected>,
}

#[derive(serde::Deserialize)]
struct Expected {
    rule: String,
    file: String,
    start: u32,
    end: u32,
}

/// The cases of `code`.
fn cases_of(code: &str) -> Cases {
    toml::from_str(
        &std::fs::read_to_string(ruff().join("rules").join(code).join("cases.toml")).expect("the cases"),
    )
    .expect("cases that parse")
}

/// Runs `codes` both ways on `files` and checks they agree; answers the interpreter's emissions.
fn agree(codes: &[&str], files: &[(String, Vec<u8>)], label: &str) -> Vec<Vec<Emission>> {
    let interpreted = interpreted(codes, files);
    let fused = fused(codes, files);
    for ((path, _), (one, (other, walks, asked))) in files.iter().zip(interpreted.iter().zip(&fused)) {
        assert_eq!(other, one, "{label}: {path}");
        assert!(*walks <= 1, "{label}: {path} walked the tree {walks} times");
        let mut seen = asked.clone();
        seen.sort_unstable_by_key(|relation| format!("{relation:?}"));
        seen.dedup();
        assert_eq!(seen.len(), asked.len(), "{label}: {path} built a relation twice: {asked:?}");
    }
    interpreted
}

/// The rule's cases pass both ways: the same findings, and the ones the case expects.
fn passes(code: &str) {
    for case in cases_of(code).cases {
        let files: Vec<(String, Vec<u8>)> = case
            .files
            .iter()
            .map(|(path, text)| (path.clone(), text.as_bytes().to_vec()))
            .collect();
        let emitted = agree(&[code], &files, &format!("{code} {}", case.name));
        let mut found: Vec<(String, u32, u32)> = files
            .iter()
            .zip(&emitted)
            .flat_map(|((path, _), rules)| {
                rules
                    .iter()
                    .flat_map(move |rule| rule.places.iter().map(move |(start, end)| (path.clone(), *start, *end)))
            })
            .collect();
        found.sort();
        let mut expected: Vec<(String, u32, u32)> = case
            .expected
            .iter()
            .filter(|each| each.rule == code)
            .map(|each| (each.file.clone(), each.start, each.end))
            .collect();
        expected.sort();
        assert_eq!(found, expected, "{code} {}", case.name);
    }
}

#[test]
fn ple2510_cases_pass_both_ways() {
    passes("PLE2510");
}

#[test]
fn log009_cases_pass_both_ways() {
    passes("LOG009");
}

#[test]
fn stage_a_rules_agree_on_the_engine_fixtures_in_one_request() {
    let emitted = agree(&["PLE2510", "LOG009"], &engine_fixtures(), "engine fixtures");
    assert!(emitted.iter().all(|rules| rules.len() == 2));
}
