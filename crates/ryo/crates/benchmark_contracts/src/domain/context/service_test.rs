//! The content seal of a listed context is the one the authoring framed: its root, its sources and auxiliary files with
//! their digests, and its absences; the physical boundary is not part of it.
use super::service::fingerprint;
use super::types::ContextManifest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The physical boundary the manifest is sealed under.
type Input = &'static str;
/// The seal.
type Value = &'static str;
/// The seal has no refusal.
type Failure = ();

/// The seal Python's `case_records.expected_context_identity` frames for the manifest below.
const AUTHORED_SEAL: &str = "310af192ba424de1b143b27fdf29ce0e92a1599ccaa901e3ad1206f62b4eaa26";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the seal is the authoring's framing of the declared context"),
        word: Word::Succeeds,
        input: || "/boundary",
        expect: Expect::Value(AUTHORED_SEAL),
    },
    Case {
        name: CaseName("another physical boundary leaves the seal unchanged"),
        word: Word::Succeeds,
        input: || "/elsewhere",
        expect: Expect::Value(AUTHORED_SEAL),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject"
)]
fn subject(boundary: Input) -> Result<Value, Failure> {
    let seal = fingerprint(&ContextManifest {
        boundary: PathBuf::from(boundary),
        logical_root: PathBuf::from("pkg"),
        source_files: BTreeMap::from([(
            PathBuf::from("pkg/a.py"),
            ArtifactDigest(
                "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881".into(),
            ),
        )]),
        auxiliary_files: BTreeMap::from([(
            PathBuf::from("setup.cfg"),
            ArtifactDigest(
                "a1fce4363854ff888cff4b8e7875d600c2682390412a8cf79b37d0b11148b0fa".into(),
            ),
        )]),
        missing: vec![PathBuf::from("pkg/b.py")],
    });
    Ok(if seal.0 == AUTHORED_SEAL {
        AUTHORED_SEAL
    } else {
        "another seal"
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
