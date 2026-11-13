//! A worker runs only the target and phases its executable expects.
use super::service::check;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::worker::fixtures::data::request;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, ParserConfigurationName, ParserSelection, RuleLabel,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Matching,
    UnpinnedRule,
    Retargeted,
    Unspecified,
    NoPhases,
    ContradictedPin,
    UnreviewedPolicy,
}
type Input = Scenario;
type Value = bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Parser,
    Selection,
    Phase,
    Unexpected,
}
type Failure = Refusal;

const TARGET: &str = "python-target=3.10";
const OTHER_TARGET: &str = "python-target=3.11";
const PINNED_RULE: &str = "UP045";
const FREE_RULE: &str = "EXE004";
const ALWAYS: &str = "always";
const ACCEPTED: &str = "accepted-syntax";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the executable's own target and the pinned phases are accepted"),
        word: Word::Succeeds,
        input: || Scenario::Matching,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a rule the language has not pinned may run in any admitted phase"),
        word: Word::Succeeds,
        input: || Scenario::UnpinnedRule,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("another target than the executable's is refused"),
        word: Word::Refuses,
        input: || Scenario::Retargeted,
        expect: Expect::Error(Refusal::Parser),
    },
    Case {
        name: CaseName("a request without a target is refused"),
        word: Word::Refuses,
        input: || Scenario::Unspecified,
        expect: Expect::Error(Refusal::Parser),
    },
    Case {
        name: CaseName("a request without selected rule phases is refused"),
        word: Word::Refuses,
        input: || Scenario::NoPhases,
        expect: Expect::Error(Refusal::Selection),
    },
    Case {
        name: CaseName("a phase that contradicts the pinned one is refused"),
        word: Word::Refuses,
        input: || Scenario::ContradictedPin,
        expect: Expect::Error(Refusal::Phase),
    },
    Case {
        name: CaseName("a phase outside the admitted policies is refused"),
        word: Word::Refuses,
        input: || Scenario::UnreviewedPolicy,
        expect: Expect::Error(Refusal::Phase),
    },
];

fn label(rule: &str) -> RuleLabel {
    RuleLabel(rule.into())
}

fn policy(text: &str) -> ApplicabilityPolicy {
    ApplicabilityPolicy(text.into())
}

fn refusal(message: &str) -> Refusal {
    use super::constants::{
        WORKER_PARSER_CONFIGURATION_DIFFER, WORKER_REQUIRES_EXPLICIT_SELECTED,
        WORKER_RULE_APPLICABILITY_DIFFERS,
    };
    if message.contains(WORKER_PARSER_CONFIGURATION_DIFFER) {
        Refusal::Parser
    } else if message.contains(WORKER_REQUIRES_EXPLICIT_SELECTED) {
        Refusal::Selection
    } else if message.contains(WORKER_RULE_APPLICABILITY_DIFFERS) {
        Refusal::Phase
    } else {
        Refusal::Unexpected
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut worker_request = request();
    worker_request.parser_configuration =
        ParserSelection::Named(ParserConfigurationName(TARGET.into()));
    worker_request.rule_applicability = BTreeMap::from([
        (label(PINNED_RULE), policy(ACCEPTED)),
        (label(FREE_RULE), policy(ALWAYS)),
    ]);
    match scenario {
        Scenario::Matching => {}
        Scenario::UnpinnedRule => {
            let _previous = worker_request
                .rule_applicability
                .insert(label("A001"), policy(ACCEPTED));
        }
        Scenario::Retargeted => {
            worker_request.parser_configuration =
                ParserSelection::Named(ParserConfigurationName(OTHER_TARGET.into()));
        }
        Scenario::Unspecified => worker_request.parser_configuration = ParserSelection::Unspecified,
        Scenario::NoPhases => worker_request.rule_applicability = BTreeMap::new(),
        Scenario::ContradictedPin => {
            let _previous = worker_request
                .rule_applicability
                .insert(label(PINNED_RULE), policy(ALWAYS));
        }
        Scenario::UnreviewedPolicy => {
            let _previous = worker_request
                .rule_applicability
                .insert(label(PINNED_RULE), policy("unreviewed"));
        }
    }
    let pinned = BTreeMap::from([
        (label(PINNED_RULE), policy(ACCEPTED)),
        (label(FREE_RULE), policy(ALWAYS)),
    ]);
    check(
        &worker_request,
        &ParserConfigurationName(TARGET.into()),
        &pinned,
    )
    .map(|()| true)
    .map_err(|error| refusal(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
