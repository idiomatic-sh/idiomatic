//! The services of the `testing` part: a `Keep` whose test reads a text. The cells are words of the file; what each test
//! holds for is the operators crate's meaning, asked for a column of texts: an expression is compiled once a batch.

use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{MissingCode, UnbuiltFill};
use crate::domain::stages::service::{selected, value_at};
use crate::domain::stages::types::{BatchRow, BatchValue, RowKept, TextMeans, TextTest};
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::service::text;
use ryo_language::domain::facts::types::FileWords;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::service::{glob_any_each, matches_each};
use ryo_operators::domain::table_texts::types::TextView;
use ryo_relations::domain::relations::types::Count;

/// What an outcome of a meaning says of a row; with `wanted` the other outcome, the row is kept when the test fails.
#[inline]
fn kept(outcome: PredicateOutcome, wanted: PredicateOutcome) -> RowKept {
    if outcome == wanted {
        RowKept::Yes
    } else {
        RowKept::No
    }
}

/// `Keep` of a test of a text: the rows of the batch it holds for, as a selection. `Matches` searches each row's text with
/// its one expression; the glob test asks whether each row's text matches any of its globs, one constant or a set, or
/// the row's own; its negation keeps the other rows.
///
/// # Errors
///
/// The code names an expression it does not hold or that does not compile; or the test is one the fill side does not
/// run yet.
pub(crate) fn keep_text(
    means: TextMeans<'_>,
    words: &FileWords,
    test: TextTest<'_>,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) -> Result<(), FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Regex,
    };
    let lefts = (0..rows.0).map(|row| TextView(text(words, value_at(test.left, Count(row))).0));
    match test.test {
        TestOperation::Matches(regex) => {
            let source = means
                .regexes
                .get(usize::try_from(regex.0).unwrap_or(usize::MAX))
                .ok_or(wrong.clone())?;
            let each = matches_each(lefts, TextView(&source.0), means.compiled, means.matcher)
                .map_err(|_| wrong)?;
            selected(
                rows,
                keep,
                each.map(|outcome| kept(outcome, PredicateOutcome::Holds)),
            );
            Ok(())
        }
        TestOperation::Glob | TestOperation::NotGlob => {
            let wanted = match test.test {
                TestOperation::NotGlob => PredicateOutcome::Fails,
                _ => PredicateOutcome::Holds,
            };
            match test.right {
                Some(BatchValue::Column(globs)) => {
                    // Each row tests its text against its own glob.
                    let mut held = Vec::with_capacity(rows.0);
                    for (left, glob) in lefts.zip(globs) {
                        let one = [TextView(text(words, *glob).0)];
                        held.extend(
                            glob_any_each(std::iter::once(left), &one, means.matcher)
                                .map(|outcome| kept(outcome, wanted)),
                        );
                    }
                    selected(rows, keep, held.into_iter());
                }
                Some(BatchValue::Constant(glob)) => {
                    let one = [TextView(text(words, glob).0)];
                    selected(
                        rows,
                        keep,
                        glob_any_each(lefts, &one, means.matcher)
                            .map(|outcome| kept(outcome, wanted)),
                    );
                }
                None => {
                    let several: Vec<TextView<'_>> = test
                        .set
                        .iter()
                        .map(|glob| TextView(text(words, *glob).0))
                        .collect();
                    selected(
                        rows,
                        keep,
                        glob_any_each(lefts, &several, means.matcher)
                            .map(|outcome| kept(outcome, wanted)),
                    );
                }
            }
            Ok(())
        }
        TestOperation::StartsWith
        | TestOperation::EndsWith
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Stage(Op::StageKeep),
        }),
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn => Err(FilledError::Code {
            named: MissingCode::Word,
        }),
    }
}
