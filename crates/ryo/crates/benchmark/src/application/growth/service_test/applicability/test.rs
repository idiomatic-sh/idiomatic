//! A growth request reads the bench's whole rules directory, so it carries the applicability the bench declares for every rule,
//! and a grown rule's own profile has the last word on that rule.
use super::super::super::service::plan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::GrowthSource;
use benchmark_contracts::domain::requests::types::ParserSelection;

/// The rule whose applicability is asked.
type Input = &'static str;
/// The applicability the plan's request gives it, if any.
type Value = Option<&'static str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a rule only the bench profile declares keeps the bench's applicability"),
        word: Word::Succeeds,
        input: || "OTHER",
        expect: Expect::Value(Some("accepted-syntax")),
    },
    Case {
        name: CaseName("a grown rule takes its own profile's applicability"),
        word: Word::Succeeds,
        input: || "F401",
        expect: Expect::Value(Some("always")),
    },
];

fn subject(rule: Input) -> Result<Value, Failure> {
    let bench = tempfile::tempdir().map_err(|error| error.to_string())?;
    crate::outbound::suite::fixtures::samples::growth_bench(bench.path())
        .map_err(|error| error.to_string())?;
    let planned = plan(
        &crate::outbound::suite::factory::local(),
        GrowthSource {
            bench: bench.path(),
            rules: &bench.path().join("rules"),
            table: &bench.path().join("growth.toml"),
            parser: &ParserSelection::Unspecified,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(planned
        .request
        .rule_applicability
        .iter()
        .find(|(label, _)| label.0 == rule)
        .map(|(_, policy)| match policy.0.as_str() {
            "always" => "always",
            "accepted-syntax" => "accepted-syntax",
            _ => "other",
        }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
