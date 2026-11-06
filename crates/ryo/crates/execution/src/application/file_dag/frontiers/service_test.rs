//! Preparation leaves a stored frontier packed; its decode runs on the first demand and a
//! decode failure is the same typed refusal preparation used to record, at the same demand.
use super::{admit, PackedFrontier};
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    stored_facts::types::{KeptWords, PackedWords},
    table_words::types::DecodeMode,
};
use std::sync::Arc;

/// A varint whose continuation bit is never closed: no canonical frame spells it.
fn cut_words() -> Arc<KeptWords> {
    Arc::new(KeptWords(PackedWords(vec![0x80])))
}
fn packed(input: Input, mode: DecodeMode) -> PackedFrontier {
    PackedFrontier {
        input,
        words: cut_words(),
        mode,
        coverage: CoverageState::Complete,
    }
}

#[test]
fn a_cut_frontier_is_refused_by_its_input_when_admitted_in_complete_mode() {
    let input = Input::Relation(Relation::Function);
    let refused = admit(packed(input, DecodeMode::Complete), &mut Symbols::default())
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
    let refused = admit(packed(input, DecodeMode::Complete), &mut Symbols::default())
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
    let admitted = admit(packed(input, DecodeMode::Permissive), &mut Symbols::default())
        .ok()
        .expect("diagnostic frontiers decode what they can");
    assert!(matches!(admitted, FrontierInput::Decoded { .. }));
}
