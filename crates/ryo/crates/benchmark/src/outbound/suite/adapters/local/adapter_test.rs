//! The concrete local implementation executes the complete shared contract.
use super::adapter::SuiteLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::audit::value::Audit;
use crate::outbound::suite::fixtures::contract::check;
use crate::outbound::suite::interface::SuitePort as _;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
type Input = bool;
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concrete implementation answers the shared contract"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty rules and empty case evidence cannot certify"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(()),
    },
];
fn subject(selected: Input) -> Result<Value, Failure> {
    if !selected {
        empty_cases_are_refused();
        return Err(());
    }
    check(&SuiteLocalAdapter);
    let directory = tempfile::tempdir().expect("source artifacts");
    let bytes = directory.path().join("executable");
    std::fs::write(&bytes, b"abc").expect("digest fixture");
    assert_eq!(
        SuiteLocalAdapter
            .digest(ArtifactPath(&bytes))
            .expect("digest")
            .0,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        matches!(SuiteLocalAdapter.digest(ArtifactPath(directory.path())), Err(crate::domain::diagnostics::errors::DiagnosticsError::Invalid(message)) if message == "executable digest requires a regular file")
    );
    let destination = tempfile::tempdir().expect("snapshot parent");
    selected_snapshot(directory.path(), destination.path());
    negative_only_evidence(directory.path());
    workload_snapshot(directory.path(), destination.path());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn empty_cases_are_refused() {
    for empty_rule in [false, true] {
        let rules = tempfile::tempdir().expect("rules");
        let scratch = tempfile::tempdir().expect("scratch");
        if empty_rule {
            let directory = rules.path().join("fixture");
            std::fs::create_dir_all(&directory).expect("rule");
            std::fs::write(
                directory.join("query.ryoql"),
                "let rows = select * from comment; emit rows subject file key(span);",
            )
            .expect("query");
            std::fs::write(directory.join("cases.toml"), "cases = []").expect("cases");
        }
        let result =
            SuiteLocalAdapter.cases(ArtifactPath(rules.path()), ArtifactPath(scratch.path()));
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

#[expect(
    clippy::too_many_lines,
    reason = "the test helper stages one complete profile snapshot in one controlled fixture"
)]
fn selected_snapshot(directory: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(directory.join("fragments")).expect("fragment directory");
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "fragment source",
    )
    .expect("fragment source");
    std::fs::write(
        directory.join("benchmark.toml"),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['E722','F704']\n[query-modules]\n'python/ruff/suppress'='fragments/suppress.ryoql'\n",
    )
    .expect("profile");
    for rule in ["E722", "F704"] {
        let root = directory.join("rules").join(rule);
        std::fs::create_dir_all(&root).expect("rule root");
        std::fs::write(root.join("query.ryoql"), rule).expect("query");
        std::fs::write(root.join("cases.toml"), "case bytes").expect("cases");
    }
    let inventory = Audit {
        implemented: vec![
            "E722".try_into().expect("rule identifier"),
            "F704".try_into().expect("rule identifier"),
        ],
        missing: vec!["F401".try_into().expect("rule identifier")],
    };
    let selected = inventory
        .select(&["E722".try_into().expect("rule identifier")])
        .expect("explicit subset");
    assert_eq!(
        selected
            .implemented
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["E722"]
    );
    assert_eq!(
        selected
            .missing
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["F401", "F704"]
    );
    let snapshot = destination.join("profile");
    SuiteLocalAdapter
        .stage(ArtifactPath(directory), ArtifactPath(&snapshot), &selected)
        .expect("staged subset");
    let children: Vec<_> = std::fs::read_dir(snapshot.join("rules"))
        .expect("snapshot rules")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(children, [std::ffi::OsString::from("E722")]);
    let fingerprint = SuiteLocalAdapter
        .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
        .expect("seal");
    assert_eq!(
        std::fs::read_to_string(snapshot.join("fragments/suppress.ryoql")).expect("staged module"),
        "fragment source"
    );
    std::fs::write(
        directory.join("fragments/suppress.ryoql"),
        "concurrent module edit",
    )
    .expect("source module edit");
    assert_eq!(
        fingerprint,
        SuiteLocalAdapter
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
            .expect("unchanged staged module seal")
    );
    std::fs::write(
        snapshot.join("fragments/suppress.ryoql"),
        "changed staged module",
    )
    .expect("staged module edit");
    assert_ne!(
        fingerprint,
        SuiteLocalAdapter
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
            .expect("changed staged module seal")
    );
    let changed_module_fingerprint = SuiteLocalAdapter
        .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
        .expect("seal after staged module edit");
    std::fs::write(directory.join("rules/F704/query.ryoql"), "concurrent work")
        .expect("unselected edit");
    assert_eq!(
        changed_module_fingerprint,
        SuiteLocalAdapter
            .fingerprint(ArtifactSet(std::slice::from_ref(&snapshot)))
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
            .fingerprint(ArtifactSet(&[snapshot]))
            .expect("changed seal")
    );
}

fn negative_only_evidence(directory: &std::path::Path) {
    let oracle = directory.join("oracle");
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
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["F621".try_into().expect("rule identifier")],
        deferred: [(
            "F621".try_into().expect("rule identifier"),
            "upstream_inactive".into(),
        )]
        .into(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
    };
    let audited = SuiteLocalAdapter
        .audit(ArtifactPath(&oracle), &tool)
        .expect("negative-only oracle evidence");
    assert!(audited.implemented.is_empty());
    assert_eq!(
        audited
            .missing
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["F621"]
    );
    std::fs::write(oracle.join("rules/F621/query.ryoql"), "nonempty query")
        .expect("authored query");
    assert_eq!(
        SuiteLocalAdapter
            .audit(ArtifactPath(&oracle), &tool)
            .expect_err("negative-only authored evidence cannot certify")
            .to_string(),
        "a rule requires positive and negative cases"
    );
}

fn workload_snapshot(directory: &std::path::Path, destination: &std::path::Path) {
    let shared = directory.join("shared");
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
    let frozen = destination.join("workload");
    SuiteLocalAdapter
        .stage_projects(ArtifactPath(&shared), ArtifactPath(&frozen))
        .expect("freeze workload");
    let seal = SuiteLocalAdapter
        .fingerprint(ArtifactSet(std::slice::from_ref(&frozen)))
        .expect("workload seal");
    std::fs::write(shared.join("patches/project.patch"), "concurrent edit").expect("source edit");
    assert_eq!(
        std::fs::read_to_string(frozen.join("patches/project.patch")).unwrap(),
        "original patch"
    );
    assert_eq!(
        seal,
        SuiteLocalAdapter
            .fingerprint(ArtifactSet(std::slice::from_ref(&frozen)))
            .unwrap()
    );
    std::fs::write(frozen.join("patches/project.patch"), "tampered snapshot")
        .expect("snapshot edit");
    assert_ne!(
        seal,
        SuiteLocalAdapter
            .fingerprint(ArtifactSet(&[frozen]))
            .unwrap()
    );
}
