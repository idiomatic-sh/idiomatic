//! A cut packed frontier is refused or decoded by its input, in the mode it is admitted in.
use super::fixtures::packed;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::table_words::types::DecodeMode;
use std::borrow::Cow;

// Preparation leaves a stored frontier packed. Its decode runs on the first demand (open of the
// project run), and a decode failure is the same typed refusal preparation used to record: it is
// consulted at the same place, the first supply of that source, so the error precedence holds.

/// What an admitted frontier became.
#[derive(Debug, PartialEq)]
enum Admitted {
    Decoded,
    Compact,
    Packed,
}

/// Why a frontier was refused: its observation is incomplete, for this reason, or another error.
#[derive(Debug, PartialEq)]
enum Refusal {
    Incomplete(Cow<'static, str>),
    Other(OperatingError),
}

/// A refused frontier: the input it was packed for, and why.
#[derive(Debug, PartialEq)]
struct Refused {
    input: PlanInput,
    refusal: Refusal,
}

/// The input a cut frontier is packed for, and the mode it is admitted in.
type Input = (PlanInput, DecodeMode);
type Value = Admitted;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode"),
        word: Word::Refuses,
        input: || (PlanInput::Relation(Relation::Function), DecodeMode::Complete),
        expect: Expect::Error(Refused {
            input: PlanInput::Relation(Relation::Function),
            refusal: Refusal::Incomplete(Cow::Borrowed(
                "encoded publication is truncated, noncanonical or has incompatible frame schemas",
            )),
        }),
    },
    Case {
        name: CaseName(
            "a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode",
        ),
        word: Word::Refuses,
        input: || (PlanInput::Step(StepId(0)), DecodeMode::Complete),
        expect: Expect::Error(Refused {
            input: PlanInput::Step(StepId(0)),
            refusal: Refusal::Incomplete(Cow::Borrowed(
                "encoded publication is truncated, noncanonical or has incompatible frame schemas",
            )),
        }),
    },
    Case {
        name: CaseName("the_permissive_diagnostic_decode_never_refuses_a_packed_frontier"),
        word: Word::Succeeds,
        input: || (PlanInput::Relation(Relation::Function), DecodeMode::Permissive),
        expect: Expect::Value(Admitted::Decoded),
    },
];

/// Admits the cut frontier packed for the input in the mode.
fn subject((input, mode): Input) -> Result<Value, Failure> {
    crate::application::file_dag::frontiers::service::admit(
        packed(input, mode),
        &mut Symbols::default(),
    )
    .map(|admitted| match admitted {
        FrontierInput::Decoded { .. } => Admitted::Decoded,
        FrontierInput::Compact { .. } => Admitted::Compact,
        FrontierInput::Packed(_) => Admitted::Packed,
    })
    .map_err(|refused| Refused {
        input: refused.input,
        refusal: match refused.error {
            OperatingError::ObservationIncomplete { reason } => {
                Refusal::Incomplete(Cow::Owned(reason))
            }
            other => Refusal::Other(other),
        },
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
