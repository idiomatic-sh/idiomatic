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

// Preparation leaves a stored frontier packed. Its decode runs on the first demand (open of the
// project run), and a decode failure is the same typed refusal preparation used to record: it is
// consulted at the same place, the first supply of that source, so the error precedence holds.

fn a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = PlanInput::Relation(Relation::Function);
    let refused = crate::application::file_dag::frontiers::service::admit(
        packed(input, DecodeMode::Complete),
        &mut Symbols::default(),
    )
    .err()
    .expect("a cut frame is never complete");
    assert_eq!(refused.input, input);
    assert!(matches!(
        refused.error,
        OperatingError::ObservationIncomplete { .. }
    ));
}

fn a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = PlanInput::Step(StepId(0));
    let refused = crate::application::file_dag::frontiers::service::admit(
        packed(input, DecodeMode::Complete),
        &mut Symbols::default(),
    )
    .err()
    .expect("a cut frame is never complete");
    assert_eq!(refused.input, input);
    assert!(matches!(
        refused.error,
        OperatingError::ObservationIncomplete { .. }
    ));
}

fn the_permissive_diagnostic_decode_never_refuses_a_packed_frontier() {
    let input = PlanInput::Relation(Relation::Function);
    let admitted = crate::application::file_dag::frontiers::service::admit(
        packed(input, DecodeMode::Permissive),
        &mut Symbols::default(),
    )
    .expect("diagnostic frontiers decode what they can");
    assert!(matches!(admitted, FrontierInput::Decoded { .. }));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ACutFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode,
    ACutStepFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode,
    ThePermissiveDiagnosticDecodeNeverRefusesAPackedFrontier,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode"),
        word: Word::Succeeds,
        input: || Scenario::ACutFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode",
        ),
        word: Word::Succeeds,
        input: || Scenario::ACutStepFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_permissive_diagnostic_decode_never_refuses_a_packed_frontier"),
        word: Word::Succeeds,
        input: || Scenario::ThePermissiveDiagnosticDecodeNeverRefusesAPackedFrontier,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ACutFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode => {
            a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode();
            Ok(())
        }
        Scenario::ACutStepFrontierIsRefusedByItsInputWhenAdmittedInCompleteMode => {
            a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode();
            Ok(())
        }
        Scenario::ThePermissiveDiagnosticDecodeNeverRefusesAPackedFrontier => {
            the_permissive_diagnostic_decode_never_refuses_a_packed_frontier();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
