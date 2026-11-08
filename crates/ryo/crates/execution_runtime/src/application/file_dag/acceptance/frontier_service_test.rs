//! A cut packed frontier is refused or decoded by its input, in the mode it is admitted in.
use crate::application::file_dag::frontiers::types::FrontierInput;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::table_words::types::DecodeMode;
use std::sync::Arc;

// Preparation leaves a stored frontier packed. Its decode runs on the first demand (open of the
// project run), and a decode failure is the same typed refusal preparation used to record: it is
// consulted at the same place, the first supply of that source, so the error precedence holds.

/// A varint whose continuation bit is never closed: no canonical frame spells it.
fn cut_words() -> Arc<ryo_values::domain::stored_facts::types::KeptWords> {
    Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
        ryo_values::domain::stored_facts::types::PackedWords(vec![0x80]),
    ))
}
fn packed(
    input: Input,
    mode: DecodeMode,
) -> crate::application::file_dag::frontiers::types::PackedFrontier {
    crate::application::file_dag::frontiers::types::PackedFrontier {
        input,
        words: cut_words(),
        mode,
        coverage: CoverageState::Complete,
    }
}

#[test]
fn a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = Input::Relation(Relation::Function);
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

#[test]
fn a_cut_step_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = Input::Step(StepId(0));
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

#[test]
fn the_permissive_diagnostic_decode_never_refuses_a_packed_frontier() {
    let input = Input::Relation(Relation::Function);
    let admitted = crate::application::file_dag::frontiers::service::admit(
        packed(input, DecodeMode::Permissive),
        &mut Symbols::default(),
    )
    .ok()
    .expect("diagnostic frontiers decode what they can");
    assert!(matches!(admitted, FrontierInput::Decoded { .. }));
}
