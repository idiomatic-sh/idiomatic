//! Missing implementations remain visible and an empty selection is not coverage.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use crate::outbound::suite::interface::SuitePort;
type Input = bool;
type Value = (usize, usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The reference default inventory is empty.
    EmptyDefaults,
    /// The implementation returned an unexpected failure category.
    Unexpected,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing implementation stays missing"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("empty inventory is refused"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Failure::EmptyDefaults),
    },
];
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
pub fn check(adapter: &impl SuitePort) {
    let temporary = tempfile::tempdir().expect("suite fixture");
    std::fs::create_dir(temporary.path().join("rules")).expect("rules fixture");
    let subject = |selected| {
        let tool = Tool {
            tool: "fixture".into(),
            version: "1".into(),
            sha256: String::new(),
            defaults: if selected {
                vec!["fixture".into()]
            } else {
                Vec::new()
            },
        };
        adapter
            .audit(temporary.path(), &tool)
            .map(|audit| (audit.implemented.len(), audit.missing.len()))
            .map_err(|error| match error {
                Error::Invalid(message)
                    if message == "empty or duplicated default rule inventory" =>
                {
                    Failure::EmptyDefaults
                }
                _ => Failure::Unexpected,
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;

/// Produce the acceptance fixture through the real artifact boundary.
pub fn audited(complete: bool) -> crate::domain::suite::types::Audit {
    let directory = tempfile::tempdir().expect("audit fixture");
    std::fs::create_dir(directory.path().join("rules")).expect("rule root");
    if complete {
        let rule = directory.path().join("rules/fixture");
        std::fs::create_dir(&rule).expect("rule directory");
        std::fs::write(
            rule.join("query.ryoql"),
            "let nodes = select * from \"syntax-tree\"; emit nodes subject file key(id);",
        )
        .expect("query");
        std::fs::write(
            rule.join("cases.toml"),
            r#"[[cases]]
name = "positive"
files = { "input.txt" = "x" }
expected = [{file="input.txt",start=0,end=1}]
[[cases]]
name = "negative"
files = { "input.txt" = "" }
expected = []
"#,
        )
        .expect("cases");
    }
    let tool = Tool {
        tool: "fixture".into(),
        version: "1".into(),
        sha256: String::new(),
        defaults: vec!["fixture".into()],
    };
    crate::outbound::suite::factory::local()
        .audit(directory.path(), &tool)
        .expect("audited fixture")
}
