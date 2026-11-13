//! A tally counts the tool's defaults, deferrals and pending rules beside the authored total.
use super::value::RuleTally;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::requests::types::{
    ParserSelection, ProviderSelection, SearchRoots,
};
use ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount;
use ryo_benchmark_contracts::domain::workloads::types::{
    FileObservation, RuleContextProfile, Tool,
};

/// The tally, rendered as text.
type Value = &'static str;

const CASES: &[Case<(), Value, ()>] = &[Case {
    name: CaseName("five defaults, one deferred, one pending and two authored are counted"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(
        "RuleTally { defaults: RuleCount(5), deferred: RuleCount(1), pending: RuleCount(1), authored: RuleCount(2) }",
    ),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        let rule = |id| ryo::linter::RuleId::try_from(id).map_err(|_invalid| ());
        let tool = Tool {
            rule_observation_selection: ObservationSelectionPolicy::Declared,
            supporting_rules: Vec::new(),
            rule_observations: Default::default(),
            rule_context: RuleContextProfile::Undeclared,
            file_observation: FileObservation::Undeclared,
            project_context_mode: Default::default(),
            tool: "fixture".into(),
            version: "1".into(),
            sha256: vec!["pin".into()],
            defaults: vec![rule("A")?, rule("B")?, rule("C")?, rule("D")?, rule("E")?],
            deferred: [(rule("C")?, "group".into())].into(),
            pending: [(rule("D")?, "group".into())].into(),
            query_modules: Default::default(),
            rule_applicability: Default::default(),
            parser_configuration: ParserSelection::Unspecified,
            source_language: ProviderSelection::Unspecified,
            project_search_roots: SearchRoots::Undeclared,
        };
        Ok(&*Box::leak(
            format!("{:?}", RuleTally::of(&tool, RuleCount(2))).into_boxed_str(),
        ))
    });
}
