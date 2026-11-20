//! The constants of the `lowering` concept: what a refusal says of a rule that is not loaded. A refusal says that a step cannot
//! be evaluated or that a count cannot be written; never that a rule costs too much or is too large.

/// What a refusal calls a place of a rule's plan.
pub(crate) const STEP: &str = "step";
/// Why a step is refused.
pub(crate) const CANNOT_BE_EVALUATED: &str = "cannot be evaluated";
/// The operands of one word, counted in sixteen bits.
pub(crate) const PAST_WORD_OPERANDS: &str = "more than 65,535 operands in one word";
/// The columns of one table or definition, counted in sixteen bits.
pub(crate) const PAST_TABLE_COLUMNS: &str = "more than 65,535 columns in one table";
/// The kinds or the fields of the vocabulary, counted in sixteen bits.
pub(crate) const PAST_VOCABULARY: &str = "more than 65,535 kinds or fields in the vocabulary";
/// The registers of the pack, counted in thirty-two bits.
pub(crate) const PAST_REGISTERS: &str = "more than 4,294,967,295 registers in the pack";
/// The words of the pack, counted in thirty-two bits.
pub(crate) const PAST_WORDS: &str = "more than 4,294,967,295 words in the pack";
/// The sites of one of the pack's tables, counted in thirty-two bits.
pub(crate) const PAST_SITES: &str = "more than 4,294,967,295 sites in the pack";
