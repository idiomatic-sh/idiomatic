//! The `project` service's cases, on values alone.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

use super::service::{reads_any, reads_project, with_consulted};
use super::types::PlanReads;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::storing::types::MadeAgain;
use ryo_values::domain::records::types::{Freshness, PlacedRule};

/// What a row of the cases does; a number packs the answers of the calls it names, first call in the highest digit.
#[derive(Clone, Copy)]
enum Scenario {
    /// `reads_project` on a frontier of the manifests, then on one of the files.
    ReadsProject,
    /// `reads_any` on plans of which one reads the project, then on plans of which none does.
    ReadsAny,
    /// `with_consulted` when the files the lookup read are as they were, whatever is made again, then when one changed, with
    /// nothing, some or every rule made again.
    WithConsulted,
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number, or what a function counts or picks.
type Value = usize;
type Failure = ();

fn plan(frontier: Vec<PlanInput>, reads: &[Relation]) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: reads
            .iter()
            .map(|relation| PlanStep {
                operation: Operation::Union {
                    left: PlanInput::Relation(*relation),
                    right: PlanInput::Step(StepId(0)),
                },
                scope: StepScope::CrossFile,
            })
            .collect(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier,
        comparison_origins: Vec::new(),
    }
}

fn manifests() -> Plan {
    plan(vec![PlanInput::Relation(Relation::PackageDependency)], &[])
}

fn files() -> Plan {
    plan(vec![PlanInput::Relation(Relation::Defines)], &[])
}

fn yes(reads: PlanReads) -> usize {
    usize::from(reads == PlanReads::Reads)
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::ReadsProject => {
            yes(reads_project(&manifests())) * 10 + yes(reads_project(&files()))
        }
        Scenario::ReadsAny => {
            yes(reads_any(&[files(), manifests()])) * 10 + yes(reads_any(&[files(), files()]))
        }
        Scenario::WithConsulted => {
            let project = [PlacedRule(1), PlacedRule(3)];
            let same = |made: MadeAgain, expected: &MadeAgain| {
                usize::from(with_consulted(made, &project, Freshness::AsItWas) == *expected)
            };
            let changed = |made: MadeAgain, expected: &MadeAgain| {
                usize::from(with_consulted(made, &project, Freshness::Changed) == *expected)
            };
            same(MadeAgain::Nothing, &MadeAgain::Nothing) * 10_000
                + same(
                    MadeAgain::These(vec![PlacedRule(2)]),
                    &MadeAgain::These(vec![PlacedRule(2)]),
                ) * 1000
                + changed(MadeAgain::Nothing, &MadeAgain::These(project.to_vec())) * 100
                + changed(
                    MadeAgain::These(vec![PlacedRule(2), PlacedRule(3)]),
                    &MadeAgain::These(vec![PlacedRule(1), PlacedRule(2), PlacedRule(3)]),
                ) * 10
                + changed(MadeAgain::All, &MadeAgain::All)
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_frontier_of_the_manifests_is_read_and_one_of_the_files_is_not"),
        word: Word::Succeeds,
        input: || Scenario::ReadsProject,
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName(
            "plans_of_which_one_reads_the_project_read_it_and_plans_of_which_none_does_do_not",
        ),
        word: Word::Succeeds,
        input: || Scenario::ReadsAny,
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("the_project_rules_are_made_again_only_when_a_file_the_lookup_read_changed"),
        word: Word::Succeeds,
        input: || Scenario::WithConsulted,
        expect: Expect::Value(11_111),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn primary_manifest_failure_marks_only_project_facts_unknown_and_keeps_its_context() {
    let partials = super::service::unread_manifest_partials(
        &[
            Relation::PackageDependency,
            Relation::Function,
            Relation::FilesystemReadFailure,
        ],
        &ryo_language::domain::files::types::FilePath("pkg/Cargo.toml".into()),
        "read denied",
    );
    assert_eq!(partials.len(), 1);
    assert_eq!(partials[0].relation, Relation::PackageDependency);
    assert_eq!(
        partials[0].at, None,
        "project incompleteness is not an error on an unrelated primary source"
    );
    assert!(partials[0].cause.0.contains("pkg/Cargo.toml"));
    assert!(partials[0].cause.0.contains("read denied"));
}
