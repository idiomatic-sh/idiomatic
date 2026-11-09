//! The failures the lint answers for what its stages refuse: an engine refusal inside a file's visit, one in the cross-file
//! phase, and a named publication the request did not admit.
use crate::application::cross_file::completion::types::CrossedWithPublications;
use crate::application::cross_file::errors::CrossFileError;
use crate::application::cross_file::types::{OrdinaryOutcome, ProjectOutcome};
use crate::application::file_analysis::errors::FileAnalysisError;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::answering::types::{Answered, Crossed, Todo};
use crate::domain::render::types::Rendered;
use crate::inbound::use_cases::lint::errors::LintError;
use crate::inbound::use_cases::lint::service::settled;
use ryo_execution::inbound::use_cases::execution::interface::{
    OperatingError, PublicationSlot, RuleIndex,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// Which stage of the lint refuses.
#[derive(Clone, Copy, Debug)]
enum Refusal {
    /// The engine refuses inside a file's visit.
    FileVisit,
    /// The engine refuses in the cross-file phase.
    CrossFilePhase,
    /// An ordinary project rule hands a named publication the request did not admit.
    UnadmittedPublication,
    /// An ordinary project rule hands no named publication.
    NoPublication,
}

type Input = Refusal;
type Value = ();
type Failure = LintError;

fn subject(refusal: Input) -> Result<Value, Failure> {
    match refusal {
        Refusal::FileVisit => Err(LintError::from(FileAnalysisError::Execution(
            OperatingError::NotRun,
        ))),
        Refusal::CrossFilePhase => Err(LintError::from(CrossFileError::Operating(
            OperatingError::NotRun,
        ))),
        Refusal::UnadmittedPublication | Refusal::NoPublication => {
            let crossed = Crossed {
                observations: matches!(refusal, Refusal::UnadmittedPublication).then(|| {
                    CrossPublicationTransfer {
                        worker: WorkerNumber(0),
                        arena: ArenaBytes::default(),
                        roots: Vec::new(),
                    }
                }),
                ..Crossed::default()
            };
            let ordinary = OrdinaryOutcome {
                outcomes: vec![ProjectOutcome {
                    todo: Todo {
                        rule: RuleIndex(0),
                        id: RuleId("named".into()),
                    },
                    result: Ok(CrossedWithPublications {
                        slot: PublicationSlot::Unused,
                        crossed,
                        publications: Vec::new(),
                    }),
                }],
                refused: None,
                finished: Ok(()),
                symbols: Symbols::default(),
            };
            let mut answered = Answered {
                unknowns: Vec::new(),
                later: Rendered::default(),
                fed: vec![Vec::new()],
            };
            settled(ordinary, &mut answered)
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ordinary_phase_that_hands_no_named_publication_settles"),
        word: Word::Succeeds,
        input: || Refusal::NoPublication,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_engine_refusal_inside_a_file_visit_is_the_lint_execution_failure"),
        word: Word::Fails,
        input: || Refusal::FileVisit,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::Execution { .. })))
        }),
    },
    Case {
        name: CaseName("an_engine_refusal_in_the_cross_file_phase_is_an_observation_failure"),
        word: Word::Fails,
        input: || Refusal::CrossFilePhase,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                LintError::ObservationExecution { .. }
            )))
        }),
    },
    Case {
        name: CaseName("a_named_publication_the_request_did_not_admit_is_refused"),
        word: Word::Refuses,
        input: || Refusal::UnadmittedPublication,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                LintError::ObservationBindings { .. }
            )))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
