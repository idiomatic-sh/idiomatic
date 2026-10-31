//! The concrete local implementation executes the complete shared contract.
use super::adapter::SuiteLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&SuiteLocalAdapter);
    use crate::domain::suite::types::audit::value::Audit;
    use crate::outbound::suite::interface::SuitePort;
    let directory = tempfile::tempdir().expect("source artifacts");
    let destination = tempfile::tempdir().expect("snapshot parent");
    std::fs::write(directory.path().join("benchmark.toml"), "profile bytes").expect("profile");
    for rule in ["E722", "F704"] {
        let root = directory.path().join("rules").join(rule);
        std::fs::create_dir_all(&root).expect("rule root");
        std::fs::write(root.join("query.ryoql"), rule).expect("query");
        std::fs::write(root.join("cases.toml"), "case bytes").expect("cases");
    }
    let inventory = Audit {
        implemented: vec!["E722".into(), "F704".into()],
        missing: vec!["F401".into()],
    };
    let selected = inventory.select(&["E722".into()]).expect("explicit subset");
    assert_eq!(selected.implemented, ["E722"]);
    assert_eq!(selected.missing, ["F401", "F704"]);
    let snapshot = destination.path().join("profile");
    SuiteLocalAdapter
        .stage(directory.path(), &snapshot, &selected)
        .expect("staged subset");
    let children: Vec<_> = std::fs::read_dir(snapshot.join("rules"))
        .expect("snapshot rules")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(children, [std::ffi::OsString::from("E722")]);
    let fingerprint = SuiteLocalAdapter
        .fingerprint(&[snapshot.clone()])
        .expect("seal");
    std::fs::write(
        directory.path().join("rules/F704/query.ryoql"),
        "concurrent work",
    )
    .expect("unselected edit");
    assert_eq!(
        fingerprint,
        SuiteLocalAdapter
            .fingerprint(&[snapshot.clone()])
            .expect("unchanged seal")
    );
    std::fs::write(
        snapshot.join("rules/E722/query.ryoql"),
        "changed selected rule",
    )
    .expect("selected edit");
    assert_ne!(
        fingerprint,
        SuiteLocalAdapter
            .fingerprint(&[snapshot])
            .expect("changed seal")
    );
    let oracle = directory.path().join("oracle");
    std::fs::create_dir_all(oracle.join("rules/F621")).expect("oracle rule");
    std::fs::write(
        oracle.join("rules/F621/cases.toml"),
        r#"[[cases]]
name = "upstream inactive"
files = { "input.py" = "a = 1" }
expected = []
"#,
    )
    .expect("oracle cases");
    let tool = crate::domain::suite::types::Tool {
        tool: "fixture".into(),
        version: "1".into(),
        sha256: "pin".into(),
        defaults: vec!["F621".into()],
        deferred: [("F621".into(), "upstream_inactive".into())].into(),
        pending: Default::default(),
    };
    let audited = SuiteLocalAdapter
        .audit(&oracle, &tool)
        .expect("negative-only oracle evidence");
    assert!(audited.implemented.is_empty());
    assert_eq!(audited.missing, ["F621"]);
    std::fs::write(oracle.join("rules/F621/query.ryoql"), "nonempty query")
        .expect("authored query");
    assert!(
        SuiteLocalAdapter.audit(&oracle, &tool).is_err(),
        "negative-only authored evidence cannot certify"
    );
    let shared = directory.path().join("shared");
    std::fs::create_dir_all(shared.join("patches")).expect("patch directory");
    std::fs::write(
        shared.join("benchmark.toml"),
        r#"language = "fixture"
[[projects]]
name = "project"
repository = "https://example.invalid/project.git"
revision = "revision"
patch = "patches/project.patch"
"#,
    )
    .expect("shared manifest");
    std::fs::write(shared.join("patches/project.patch"), "original patch").expect("patch");
    let frozen = destination.path().join("workload");
    SuiteLocalAdapter
        .stage_projects(&shared, &frozen)
        .expect("freeze workload");
    let seal = SuiteLocalAdapter
        .fingerprint(&[frozen.clone()])
        .expect("workload seal");
    std::fs::write(shared.join("patches/project.patch"), "concurrent edit").expect("source edit");
    assert_eq!(
        std::fs::read_to_string(frozen.join("patches/project.patch")).unwrap(),
        "original patch"
    );
    assert_eq!(
        seal,
        SuiteLocalAdapter.fingerprint(&[frozen.clone()]).unwrap()
    );
    std::fs::write(frozen.join("patches/project.patch"), "tampered snapshot")
        .expect("snapshot edit");
    assert_ne!(seal, SuiteLocalAdapter.fingerprint(&[frozen]).unwrap());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    #[cfg(unix)]
    run(PIN_CASES, Word::Succeeds, verify_pin);
}

#[test]
fn refuses() {
    #[cfg(unix)]
    run(PIN_CASES, Word::Refuses, verify_pin);
    use crate::outbound::suite::interface::SuitePort;
    for empty_rule in [false, true] {
        let rules = tempfile::tempdir().expect("rules");
        let scratch = tempfile::tempdir().expect("scratch");
        if empty_rule {
            let directory = rules.path().join("fixture");
            std::fs::create_dir(&directory).expect("rule");
            std::fs::write(
                directory.join("query.ryoql"),
                "let rows = select * from comment; emit rows subject file key(span);",
            )
            .expect("query");
            std::fs::write(directory.join("cases.toml"), "cases = []").expect("cases");
        }
        let result = SuiteLocalAdapter.cases(rules.path(), scratch.path());
        let actual = result
            .expect_err("no cases cannot certify correctness")
            .to_string();
        let expected = if empty_rule {
            "case evidence must be nonempty"
        } else {
            "no executable rule cases"
        };
        assert_eq!(actual, expected);
    }
}

#[cfg(unix)]
const PIN_CASES: &[Case<(&str, &str, bool), (), &str>] = &[
    Case {
        name: CaseName("matching executable bytes and version are accepted"),
        word: Word::Succeeds,
        input: || {
            (
                "b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b",
                "1.0",
                true,
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("wrong digest refuses before attempting a non executable file"),
        word: Word::Refuses,
        input: || ("wrong", "1.0", false),
        expect: Expect::Error("hash"),
    },
    Case {
        name: CaseName("matching bytes do not bypass the expected version"),
        word: Word::Refuses,
        input: || {
            (
                "b145a69e05e755a39d063badddeffe112a67d4f55993e373cc4db2074b984c0b",
                "2.0",
                true,
            )
        },
        expect: Expect::Error("version"),
    },
];

#[cfg(unix)]
fn verify_pin((sha256, version, executable): (&str, &str, bool)) -> Result<(), &'static str> {
    use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
    use crate::outbound::suite::interface::SuitePort;
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().expect("tool directory");
    let path = directory.path().join("fixture");
    std::fs::write(&path, b"#!/bin/sh\necho fixture 1.0\n").expect("fixture bytes");
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
    )
    .expect("fixture mode");
    let tool = Tool {
        tool: "fixture".into(),
        version: version.into(),
        sha256: sha256.into(),
        deferred: Default::default(),
        pending: Default::default(),
        defaults: vec!["E722".into()],
    };
    SuiteLocalAdapter
        .verify_tool(&path, &tool)
        .map_err(|error| match error {
            Error::Invalid(message) if message == "tool executable hash differs from pin" => "hash",
            Error::Invalid(message) if message == "tool version differs from pin" => "version",
            _ => "unexpected error",
        })
}
