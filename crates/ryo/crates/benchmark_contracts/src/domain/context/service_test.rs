//! The content seal of a listed context is the one the authoring framed: its root, its sources and auxiliary files with
//! their digests, and its absences; the physical boundary is not part of it.
use super::service::fingerprint;
use super::types::{ContextManifest, LinkTarget};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The physical boundary the manifest is sealed under, and the links it seals: each a path and its target text.
type Input = (&'static str, &'static [(&'static str, &'static str)]);
/// The seal.
type Value = &'static str;
/// The seal has no refusal.
type Failure = ();

/// The seal Python's `case_records.expected_context_identity` frames for the manifest below.
const AUTHORED_SEAL: &str = "310af192ba424de1b143b27fdf29ce0e92a1599ccaa901e3ad1206f62b4eaa26";
/// The seals of the manifest below with links, each framed by an independent Python spelling of the same framing (one
/// that also reproduces AUTHORED_SEAL): every link is `link`, its path and its exact target text, after the absences.
const LINKED_SEAL: &str = "db7b9bf2f9b131df09de10f87ad1746be2c468c8a44a8c88967f227c1b3cbf38";
const OTHER_PATH_SEAL: &str = "44f45c1666d903d8cec0526420ca1fd08df7212321a770407a987d55b26936f0";
const OTHER_TARGET_SEAL: &str = "67561542f27e95271ea2eaf5fd3e116d67fec32c6bd1b8f811915bf82e9b9bba";
const EQUIVALENT_SPELLING_SEAL: &str =
    "f1514adf6c4f8bfcae54a2028a7764ab83bfafc8325eb6d7f1f94878fbc79c36";
const TWO_LINKS_SEAL: &str = "1a5cb645139e9754d4dccd5896e2ab08679944695af571a348093cf48a952512";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the seal is the authoring's framing of the declared context"),
        word: Word::Succeeds,
        input: || ("/boundary", &[]),
        expect: Expect::Value(AUTHORED_SEAL),
    },
    Case {
        name: CaseName("another physical boundary leaves the seal unchanged"),
        word: Word::Succeeds,
        input: || ("/elsewhere", &[]),
        expect: Expect::Value(AUTHORED_SEAL),
    },
    Case {
        name: CaseName("a sealed link is framed by its path and its exact target text"),
        word: Word::Succeeds,
        input: || ("/boundary", &[("pkg/c.py", "a.py")]),
        expect: Expect::Value(LINKED_SEAL),
    },
    Case {
        name: CaseName("a link at another path seals differently"),
        word: Word::Succeeds,
        input: || ("/boundary", &[("pkg/d.py", "a.py")]),
        expect: Expect::Value(OTHER_PATH_SEAL),
    },
    Case {
        name: CaseName("a link to another target seals differently"),
        word: Word::Succeeds,
        input: || ("/boundary", &[("pkg/c.py", "b.py")]),
        expect: Expect::Value(OTHER_TARGET_SEAL),
    },
    Case {
        name: CaseName("an equivalent spelling of the target seals differently"),
        word: Word::Succeeds,
        input: || ("/boundary", &[("pkg/c.py", "./a.py")]),
        expect: Expect::Value(EQUIVALENT_SPELLING_SEAL),
    },
    Case {
        name: CaseName("an additional link seals differently"),
        word: Word::Succeeds,
        input: || ("/boundary", &[("pkg/c.py", "a.py"), ("pkg/e.py", "a.py")]),
        expect: Expect::Value(TWO_LINKS_SEAL),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject"
)]
fn subject((boundary, links): Input) -> Result<Value, Failure> {
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
        links: links
            .iter()
            .map(|(path, target)| (PathBuf::from(path), LinkTarget((*target).into())))
            .collect(),
    });
    Ok(Box::leak(seal.0.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
