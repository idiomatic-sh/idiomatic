//! A rule's instance carries its source identity and the effective settings of its staged profile.
use super::instance;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::types::RuleInstanceRecord;
use ryo_benchmark_contracts::domain::documents::types::text::value::{PathText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::requests::types::ParserSelection;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// The staged profile's text.
type Input = &'static str;
/// The encoded instance.
type Value = Cow<'static, str>;
/// The refusal.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the effective settings name the rule, its applicability and its context mode"),
        word: Word::Succeeds,
        input: || "defaults = [\"A1\"]\n\n[rule-applicability]\nA1 = \"always\"\n",
        expect: Expect::Value(Cow::Borrowed("{\"case_count\":1,\"cases_file\":\"rules/A1/cases.toml\",\"cases_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"effective_settings\":{\"defaults\":[\"A1\"],\"flags\":{},\"performance\":{\"diagnostics\":{\"execution\":false,\"joins\":false,\"prefixes\":false,\"resource_routes\":false,\"row_progress\":false,\"selection_detail\":false,\"steps\":false,\"subplans\":false}},\"project_context_mode\":\"isolated\",\"rule_applicability\":{\"A1\":\"always\"}},\"evidence_source_root\":\"bench\",\"module_sha256\":{},\"profile_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"query_file\":\"rules/A1/query.ryoql\",\"query_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"settings_source\":\"bench/benchmark.toml\",\"settings_source_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\"}")),
    },
    Case {
        name: CaseName("an unreviewed execution setting is refused"),
        word: Word::Refuses,
        input: || "defaults = [\"A1\"]\nthreads = 4\n",
        expect: Expect::Error(Cow::Borrowed("unreviewed execution settings in canonical profile")),
    },
];

fn subject(profile: Input) -> Result<Value, Failure> {
    let stage = tempfile::tempdir().expect("stage");
    std::fs::write(stage.path().join("benchmark.toml"), profile).expect("profile");
    let digest = Sha256Text("0".repeat(64));
    instance(&RuleInstanceRecord {
        stage: stage.path().to_owned(),
        rule: RuleName("A1".into()),
        query_file: PathText("rules/A1/query.ryoql".into()),
        query_sha256: digest.clone(),
        cases_file: PathText("rules/A1/cases.toml".into()),
        cases_sha256: digest.clone(),
        case_count: CaseCount(1),
        settings_source: PathText("bench/benchmark.toml".into()),
        settings_source_sha256: digest.clone(),
        parser_configuration: ParserSelection::Unspecified,
        evidence_source_root: PathText("bench".into()),
        profile_sha256: digest,
        module_sha256: BTreeMap::new(),
    })
    .map(|json| Cow::Owned(json.0))
    .map_err(|error| Cow::Owned(error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
