//! The support of the `cut_refusal` cases.
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::table_words::types::DecodeMode;
use std::sync::Arc;

/// A varint whose continuation bit is never closed: no canonical frame spells it.
pub(super) fn cut_words() -> Arc<ryo_values::domain::stored_facts::types::KeptWords> {
    Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
        ryo_values::domain::stored_facts::types::PackedWords(vec![0x80]),
    ))
}

pub(super) fn packed(
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
