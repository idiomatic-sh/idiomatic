//! Observation closure follows only executable producers, and profile defaults project only what is selected.
use super::service::{closure, project_profile};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{
    ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
    ObservationKind, ObservationName, ObservationProducer, RuleId,
};
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use std::collections::BTreeMap;

/// What each scenario resolves.
#[derive(Clone, Copy)]
enum Scenario {
    ChainReachesEveryProducer,
    CycleTerminates,
    NoQueryProducerIsKnowledge,
    UnselectedProfileProjectsNothing,
    SelectedPolicyDropsUnselectedProducers,
    DeclaredPolicyKeepsDeclaredProducers,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a producer chain is reached through every link"),
        word: Word::Succeeds,
        input: || Scenario::ChainReachesEveryProducer,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a producer cycle terminates"),
        word: Word::Succeeds,
        input: || Scenario::CycleTerminates,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a rule without a query is not an executable dependency"),
        word: Word::Succeeds,
        input: || Scenario::NoQueryProducerIsKnowledge,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a profile with no selected declaration projects nothing"),
        word: Word::Succeeds,
        input: || Scenario::UnselectedProfileProjectsNothing,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the selected policy drops producers outside the cohort"),
        word: Word::Succeeds,
        input: || Scenario::SelectedPolicyDropsUnselectedProducers,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the declared policy keeps every declared producer"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredPolicyKeepsDeclaredProducers,
        expect: Expect::Value(1),
    },
];

fn id(text: &str) -> RuleId {
    RuleId::try_from(text).unwrap()
}

fn output(producer: &str) -> ObservationProducer {
    ObservationProducer::Output {
        rule: id(producer),
        output: ObservationName("matched".into()),
    }
}

fn binding(producer: ObservationProducer) -> ObservationBinding {
    ObservationBinding {
        name: ObservationBindingName("published".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: ObservationKind::File,
        }],
        producers: vec![producer],
    }
}

fn bindings(edges: &[(&str, ObservationProducer)]) -> BTreeMap<RuleId, Vec<ObservationBinding>> {
    edges
        .iter()
        .map(|(rule, producer)| (id(rule), vec![binding(producer.clone())]))
        .collect()
}

fn reached(edges: &[(&str, ObservationProducer)], seed: &str) -> usize {
    let table = bindings(edges);
    closure([id(seed)], |rule| table.get(rule).map(Vec::as_slice)).len()
}

fn projected(
    edges: &[(&str, ObservationProducer)],
    selected: &[&str],
    policy: ObservationSelectionPolicy,
) -> usize {
    let declarations = RuleObservationDeclarations::from(bindings(edges));
    let selected = selected.iter().map(|rule| id(rule)).collect::<Vec<_>>();
    project_profile(&declarations, &selected, policy)
        .values()
        .map(|rows| rows.iter().map(|row| row.producers.len()).sum::<usize>())
        .sum()
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::ChainReachesEveryProducer => {
            reached(&[("A", output("B")), ("B", output("C"))], "A")
        }
        Scenario::CycleTerminates => reached(&[("A", output("B")), ("B", output("A"))], "A"),
        Scenario::NoQueryProducerIsKnowledge => reached(
            &[("A", ObservationProducer::NoQuery { rule: id("B") })],
            "A",
        ),
        Scenario::UnselectedProfileProjectsNothing => projected(
            &[("A", output("B"))],
            &["C"],
            ObservationSelectionPolicy::Declared,
        ),
        Scenario::SelectedPolicyDropsUnselectedProducers => projected(
            &[("A", output("B"))],
            &["A"],
            ObservationSelectionPolicy::Selected,
        ),
        Scenario::DeclaredPolicyKeepsDeclaredProducers => projected(
            &[("A", output("B"))],
            &["A"],
            ObservationSelectionPolicy::Declared,
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
