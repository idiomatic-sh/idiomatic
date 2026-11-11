//! One source pass and one frontier decode per population across the rules of a cohort, and
//! retirement on every exit.
use super::super::fixtures::PhaseExit;
use super::fixtures::{Admissions, Observed, Stored, admissions, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;

/// What a row runs: one batch of the four project readers over these source values, stored so,
/// ending with this exit; or the walk's admission of the readers' first shared head.
#[derive(Clone, Copy, Debug)]
enum Sharing {
    Batch {
        exit: PhaseExit,
        values: [i64; 4],
        stored: Stored,
    },
    HeadAdmission,
}

/// What the batch's walk did, or what the head admissions answered.
#[derive(Debug, PartialEq)]
enum Shared {
    Batch(Observed),
    Admissions(Admissions),
}

type Input = Sharing;
type Value = Shared;
type Failure = OperatingError;

/// A batch that delivered these rules, refused or not, in these passes (and those before the
/// second cohort), walking these source rows, with nothing retained, and these decodes.
const fn walked(
    delivered: usize,
    refused: bool,
    passes: u64,
    passes_before_second_cohort: Option<u64>,
    source_rows: u64,
    decodes: usize,
) -> Observed {
    Observed {
        delivered,
        refused,
        passes,
        passes_before_second_cohort,
        source_rows,
        retained: 0,
        decodes,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_population_is_read_once_for_every_rule_of_its_cohort"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::Complete,
            values: [2, 2, 2, 2],
            stored: Stored::Decoded,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 1, None, 2, 0))),
    },
    Case {
        name: CaseName("a_cohort_of_a_second_population_reads_its_own_source"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::Complete,
            values: [2, 2, 8, 8],
            stored: Stored::Decoded,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 2, Some(1), 4, 0))),
    },
    Case {
        name: CaseName("retirement_follows_a_failing_terminal_result"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::SemanticFailure,
            values: [2, 2, 2, 2],
            stored: Stored::Decoded,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 1, None, 2, 0))),
    },
    Case {
        name: CaseName("retirement_follows_a_cancelled_batch"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::RefuseLaterCohort,
            values: [2, 2, 8, 8],
            stored: Stored::Decoded,
        },
        expect: Expect::Value(Shared::Batch(walked(2, true, 1, Some(1), 2, 0))),
    },
    Case {
        name: CaseName("one_population_is_decoded_once_for_every_rule_of_its_cohort"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::Complete,
            values: [2, 2, 2, 2],
            stored: Stored::Packed,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 1, None, 2, 1))),
    },
    Case {
        name: CaseName("a_second_population_is_decoded_on_its_own"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::Complete,
            values: [2, 2, 8, 8],
            stored: Stored::Packed,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 2, Some(1), 4, 2))),
    },
    Case {
        name: CaseName("decoded_rows_are_never_decoded_again"),
        word: Word::Succeeds,
        input: || Sharing::Batch {
            exit: PhaseExit::Complete,
            values: [2, 2, 2, 2],
            stored: Stored::Decoded,
        },
        expect: Expect::Value(Shared::Batch(walked(4, false, 1, None, 2, 0))),
    },
    Case {
        name: CaseName("a_head_is_admitted_only_for_a_declared_root"),
        word: Word::Succeeds,
        input: || Sharing::HeadAdmission,
        expect: Expect::Value(Shared::Admissions(Admissions {
            undeclared: Some(OperatingError::NotRead),
            declared: None,
            delivered: Some(OperatingError::NotRead),
        })),
    },
];

/// Runs the batch through one publication phase, or admits the head through a fresh walk.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Sharing::Batch {
            exit,
            values,
            stored,
        } => Ok(Shared::Batch(exercise(exit, values, stored))),
        Sharing::HeadAdmission => admissions().map(Shared::Admissions),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
