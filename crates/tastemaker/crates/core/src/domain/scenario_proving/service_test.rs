//! What the `scenario_proving` service proves a definition's cases and scenarios with, on values alone.

use super::service::{covered, entry_head, explained, subject_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::Why;
use crate::domain::entries::types::Description;
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, PackageName, Release, VersionText,
};
use crate::domain::proofs::types::{DefinitionLabel, JudgedScenario};
use crate::domain::scenarios::types::{Expectation, ScenarioId};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// A definition whose description and why are blank.
    Unexplained,
    /// A definition that says both.
    Explained,
    /// Scenarios that repeat an id, and that never fail.
    Covered,
    /// A spec's version: its id and directory, and the definition its scenarios' entry names.
    Subject,
}

type Input = Scenario;
/// What the service answered, written out.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_blank_description_and_why_are_two_problems"),
        word: Word::Succeeds,
        input: || Scenario::Unexplained,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_definition_that_explains_itself_has_no_problem"),
        word: Word::Succeeds,
        input: || Scenario::Explained,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("scenarios_that_repeat_an_id_or_never_fail_are_problems"),
        word: Word::Succeeds,
        input: || Scenario::Covered,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_version_is_named_and_placed_by_its_release"),
        word: Word::Succeeds,
        input: || Scenario::Subject,
        expect: Expect::Value(String::new()),
    },
];

/// What the scenario gave, and what it should give.
fn said(scenario: Scenario) -> (String, &'static str) {
    match scenario {
        Scenario::Unexplained | Scenario::Explained => {
            let text = match scenario {
                Scenario::Unexplained => " ",
                Scenario::Explained | Scenario::Covered | Scenario::Subject => "said",
            };
            let mut problems = Vec::new();
            explained(&mut problems, &Description(text.into()), &Why(text.into()));
            (
                problems
                    .iter()
                    .map(|problem| &*problem.0)
                    .collect::<Vec<_>>()
                    .join("|"),
                match scenario {
                    Scenario::Unexplained => "its `description` is blank|its `why` is blank",
                    Scenario::Explained | Scenario::Covered | Scenario::Subject => "",
                },
            )
        }
        Scenario::Subject => {
            let release = Release {
                package: PackageName("p".into()),
                kind: DefinitionKind::Spec,
                name: DefinitionName("s".into()),
                version: VersionText("0.1.0".into()),
                ownership: Ownership::Shared,
            };
            let subject = subject_of(&release);
            let head = entry_head(&release);
            (
                format!(
                    "{} {} | {:?} {} {}",
                    subject.id.0, subject.directory.0, head.kind, head.package.0, head.name.0
                ),
                "p/spec/s@0.1.0 specs/s/0.1.0 | Spec p s",
            )
        }
        Scenario::Covered => {
            let id = ScenarioId("todo-in-comment".into());
            let mut problems = Vec::new();
            covered(
                &mut problems,
                &DefinitionLabel("rule `r`".into()),
                &[
                    JudgedScenario {
                        id: &id,
                        verdict: Expectation::Passes,
                    },
                    JudgedScenario {
                        id: &id,
                        verdict: Expectation::Passes,
                    },
                ],
            );
            (
                problems
                    .iter()
                    .map(|problem| &*problem.0)
                    .collect::<Vec<_>>()
                    .join("|"),
                "rule `r` has two scenarios with the id `todo-in-comment`: give each [[scenarios]] entry an `id` of its own|rule `r` has no failing scenario: add a [[scenarios]] entry whose `then.findings` is not empty",
            )
        }
    }
}

fn subject(scenario: Input) -> Value {
    let (said, wanted) = said(scenario);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
