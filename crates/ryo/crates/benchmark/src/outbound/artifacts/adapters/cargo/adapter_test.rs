//! The reference tool is the first of its name on the search path, the nested build is refused off Unix, and the adapter keeps the port's contract.
use super::adapter::{ArtifactsCargoAdapter, find, refusal};
use super::settings::ArtifactsCargoSettings;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::borrow::Cow;

type Input = &'static str;
type Value = Cow<'static, str>;
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the first directory of the search path holding the tool wins"),
        word: Word::Succeeds,
        input: || "fixture-tool",
        expect: Expect::Value(Cow::Borrowed("first/fixture-tool")),
    },
    Case {
        name: CaseName("a tool no directory holds is refused by name"),
        word: Word::Refuses,
        input: || "absent-tool",
        expect: Expect::Error("reference tool not found on the search path: absent-tool"),
    },
];

#[test]
fn the_reference_is_found_on_the_search_path() {
    let root = tempfile::tempdir().expect("search path fixture");
    let (first, second) = (root.path().join("first"), root.path().join("second"));
    for directory in [&first, &second] {
        std::fs::create_dir_all(directory).expect("search directory");
    }
    std::fs::write(second.join("fixture-tool"), "second").expect("second tool");
    std::fs::write(first.join("fixture-tool"), "first").expect("first tool");
    let search = std::env::join_paths([&root.path().join("empty"), &first, &second])
        .expect("search path");
    let subject = |name: Input| {
        find(&search, &ToolName::from(name))
            .map(|path| {
                Cow::Owned(
                    path.strip_prefix(root.path().canonicalize().expect("root"))
                        .expect("inside the fixture")
                        .display()
                        .to_string(),
                )
            })
            .map_err(|error| {
                if error.to_string() == "reference tool not found on the search path: absent-tool" {
                    "reference tool not found on the search path: absent-tool"
                } else {
                    "another refusal"
                }
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

#[test]
fn a_nested_build_is_refused_off_unix() {
    assert!(refusal(true).is_ok());
    assert_eq!(
        refusal(false).map_err(|error| error.to_string()),
        Err("bench operations run on Unix only: on this platform `cargo run` holds the build lock a nested build waits on".to_owned())
    );
}

#[test]
fn keeps_the_port_contract() {
    crate::outbound::artifacts::fixtures::contract::check(&ArtifactsCargoAdapter::new(
        ArtifactsCargoSettings {
            cargo: "cargo".into(),
            search_path: std::ffi::OsString::new(),
        },
    ));
}
