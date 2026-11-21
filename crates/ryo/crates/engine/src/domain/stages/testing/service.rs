//! The services of the `testing` part: a `Keep` whose test reads a text. The cells are words of the file; the meaning of
//! each test is the operators crate's.

use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::stages::types::{BatchRow, TextTest};
use crate::domain::steps::types::StepRegex;
use ryo_language::domain::facts::types::FileWords;
use ryo_relations::domain::relations::types::Count;

/// `Keep` of a test of a text: the rows of the batch it holds for, as a selection.
///
/// # Errors
///
/// The test is one the fill side does not run yet.
pub(crate) fn keep_text(
    regexes: &[StepRegex],
    words: &FileWords,
    test: TextTest<'_>,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) -> Result<(), FilledError> {
    let _unbuilt = (regexes, words, test, rows, keep);
    Err(FilledError::Unbuilt {
        unbuilt: UnbuiltFill::Stage(Op::StageKeep),
    })
}
