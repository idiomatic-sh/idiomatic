//! The services of the `testing` part: a `Keep` whose test reads a text. The cells are words of the file; what each test
//! holds for is the operators crate's meaning, asked for a column of texts: an expression is compiled once a batch.

use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{FillRuns, MissingCode, UnbuiltFill};
use crate::domain::stages::deciding::service::{by_word, kept, verdict_key};
use crate::domain::stages::service::{selected, value_at};
use crate::domain::stages::types::{BatchRow, BatchValue, RowKept, TextMeans, TextTest};
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::service::text;
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_operators::domain::comparing::collections::comparisons::affix::item::AffixItem;
use ryo_operators::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::service::glob_any_each;
use ryo_operators::domain::table_texts::types::{Datum, TextView};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::Test;

/// Whether a `Keep` runs a test: every test of two cells and of a set, the regular expression, the globs and the affixes;
/// not yet the numeric and natural orders of texts and the successor. `keep_text` asks it before it runs a test of a
/// text, so a test said to run here and not built there fails its first case.
#[must_use]
pub(crate) const fn test_runs(test: TestOperation) -> FillRuns {
    match test {
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::Matches(_)
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith => FillRuns::Runs,
        TestOperation::NumericAtLeast | TestOperation::NaturalLess | TestOperation::Successor => {
            FillRuns::NotYet
        }
    }
}

/// A glob test of each row's text against the row's own glob: nothing here is the same from row to row, so each glob is
/// read for its row. `wanted` is the outcome kept.
fn own_globs(
    means: TextMeans<'_>,
    words: &FileWords,
    test: TextTest<'_>,
    globs: &[CellWord],
    wanted: PredicateOutcome,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) {
    let lefts = (0..rows.0).map(|row| TextView(text(words, value_at(test.left, Count(row))).0));
    selected(
        rows,
        keep,
        lefts.zip(globs).map(|(left, glob)| {
            let one = [TextView(text(words, *glob).0)];
            match glob_any_each(std::iter::once(left), &one, means.matcher).next() {
                Some(outcome) => kept(outcome, wanted),
                None => RowKept::No,
            }
        }),
    );
}

/// An affix test of each row's text: the operators crate's comparison, with the row's own affix or one for every row.
///
/// # Errors
///
/// The test names no affix.
fn affixed(
    words: &FileWords,
    test: TextTest<'_>,
    asked: Test,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) -> Result<(), FilledError> {
    let lefts = (0..rows.0).map(|row| TextView(text(words, value_at(test.left, Count(row))).0));
    let mut held = Vec::with_capacity(rows.0);
    for (row, whole) in lefts.enumerate() {
        let affix = match test.right {
            Some(value) => text(words, value_at(value, Count(row))),
            None => {
                return Err(FilledError::Code {
                    named: MissingCode::Word,
                });
            }
        };
        let outcome = AffixItem.holds(asked, &Datum::Text(whole), &Datum::Text(TextView(affix.0)));
        held.push(match outcome {
            Ok(PredicateOutcome::Holds) => RowKept::Yes,
            Ok(PredicateOutcome::Fails) | Err(_) => RowKept::No,
        });
    }
    selected(rows, keep, held.into_iter());
    Ok(())
}

/// `Keep` of a test of a text: the rows of the batch it holds for, as a selection. `Matches` searches each row's text with
/// its one expression; the glob test asks whether each row's text matches any of its globs, one constant or a set, or
/// the row's own; its negation keeps the other rows; `StartsWith` and `EndsWith` ask the operators crate's affix
/// comparison whether each row's text begins or ends with its other text.
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
    if test_runs(test.test) == FillRuns::NotYet {
        return Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Stage(Op::StageKeep),
        });
    }
    // A test against constants is decided once for each word, and read for each row.
    if let Some(key) = verdict_key(test) {
        return by_word(means, words, test, key, rows, keep);
    }
    // What is left is a test whose other side is the row's own: a row's glob, a row's affix.
    let own = |wanted: PredicateOutcome| match test.right {
        Some(BatchValue::Column(globs)) => Ok((globs, wanted)),
        Some(BatchValue::Constant(_)) | None => Err(FilledError::Code {
            named: MissingCode::Word,
        }),
    };
    match test.test {
        TestOperation::Glob => {
            let (globs, wanted) = own(PredicateOutcome::Holds)?;
            own_globs(means, words, test, globs, wanted, rows, keep);
            Ok(())
        }
        TestOperation::NotGlob => {
            let (globs, wanted) = own(PredicateOutcome::Fails)?;
            own_globs(means, words, test, globs, wanted, rows, keep);
            Ok(())
        }
        TestOperation::StartsWith => affixed(words, test, Test::StartsWith, rows, keep),
        TestOperation::EndsWith => affixed(words, test, Test::EndsWith, rows, keep),
        TestOperation::NumericAtLeast | TestOperation::NaturalLess | TestOperation::Successor => {
            Err(FilledError::Unbuilt {
                unbuilt: UnbuiltFill::Stage(Op::StageKeep),
            })
        }
        // A regular expression is always against a constant: it is decided by the word, above.
        TestOperation::Matches(_)
        | TestOperation::Equal
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
