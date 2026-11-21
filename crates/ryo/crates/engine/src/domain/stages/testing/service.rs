//! The services of the `testing` part: a `Keep` whose test reads a text. The cells are words of the file; what each test
//! holds for is the operators crate's meaning, asked for a column of texts: an expression is compiled once a batch.

use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{FillRuns, MissingCode, UnbuiltFill};
use crate::domain::stages::service::{selected, value_at};
use crate::domain::stages::types::{BatchRow, BatchValue, RowKept, TextMeans, TextTest};
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::service::text;
use ryo_language::domain::facts::types::FileWords;
use ryo_operators::domain::comparing::collections::comparisons::affix::item::AffixItem;
use ryo_operators::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::service::{glob_any_each, matches_each};
use ryo_operators::domain::table_texts::types::{Datum, TextView};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::Test;

/// What an outcome of a meaning says of a row; with `wanted` the other outcome, the row is kept when the test fails.
#[inline]
fn kept(outcome: PredicateOutcome, wanted: PredicateOutcome) -> RowKept {
    if outcome == wanted {
        RowKept::Yes
    } else {
        RowKept::No
    }
}

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

/// A glob test of each row's text: against the row's own glob, one glob, or any of a set. `wanted` is the outcome kept.
fn globbed(
    means: TextMeans<'_>,
    words: &FileWords,
    test: TextTest<'_>,
    wanted: PredicateOutcome,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) {
    let lefts = (0..rows.0).map(|row| TextView(text(words, value_at(test.left, Count(row))).0));
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
                glob_any_each(lefts, &one, means.matcher).map(|outcome| kept(outcome, wanted)),
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
                glob_any_each(lefts, &several, means.matcher).map(|outcome| kept(outcome, wanted)),
            );
        }
    }
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
        TestOperation::Glob => {
            globbed(means, words, test, PredicateOutcome::Holds, rows, keep);
            Ok(())
        }
        TestOperation::NotGlob => {
            globbed(means, words, test, PredicateOutcome::Fails, rows, keep);
            Ok(())
        }
        TestOperation::StartsWith => affixed(words, test, Test::StartsWith, rows, keep),
        TestOperation::EndsWith => affixed(words, test, Test::EndsWith, rows, keep),
        TestOperation::NumericAtLeast | TestOperation::NaturalLess | TestOperation::Successor => {
            Err(FilledError::Unbuilt {
                unbuilt: UnbuiltFill::Stage(Op::StageKeep),
            })
        }
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
