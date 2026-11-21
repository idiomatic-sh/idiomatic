//! The services of the `deciding` part: which text tests are decided by the word alone, what such a test says of one
//! text (the operators crate's meaning, asked once), and the `Keep` that reads each row's word in the test's verdicts.

use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::stages::service::{selected, value_at};
use crate::domain::stages::types::{BatchRow, BatchValue, Deciding, RowKept, TextMeans, TextTest};
use crate::domain::stages::verdicts::service::{known, opened, remembered};
use crate::domain::stages::verdicts::types::{Against, TextTestKey, WordBounds, WordKnown};
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::service::{base_count, text, words_end};
use ryo_language::domain::facts::types::FileWords;
use ryo_operators::domain::comparing::collections::comparisons::affix::item::AffixItem;
use ryo_operators::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::service::{glob_compiled, globs_compiled, matches_each};
use ryo_operators::domain::patterns::types::TextGlobSet;
use ryo_operators::domain::table_texts::types::{Datum, TextView};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::Test;

/// What an outcome of a meaning says of a row; with `wanted` the other outcome, the row is kept when the test fails.
#[inline]
pub(crate) fn kept(outcome: PredicateOutcome, wanted: PredicateOutcome) -> RowKept {
    if outcome == wanted {
        RowKept::Yes
    } else {
        RowKept::No
    }
}

/// What a test's verdicts are kept by, when the test is of one text against constants: its verdict then depends on the
/// text alone. A test whose other side moves from row to row (a row's own glob, a row's own affix) has none.
pub(crate) fn verdict_key(test: TextTest<'_>) -> Option<TextTestKey> {
    let against = match (test.test, test.right, test.set_at) {
        (TestOperation::Matches(_), _, _) => Against::Nothing,
        (
            TestOperation::Glob
            | TestOperation::NotGlob
            | TestOperation::StartsWith
            | TestOperation::EndsWith,
            Some(BatchValue::Constant(cell)),
            _,
        ) => Against::Cell(cell),
        (TestOperation::Glob | TestOperation::NotGlob, None, Some(place)) => Against::Set(place),
        (
            TestOperation::Glob
            | TestOperation::NotGlob
            | TestOperation::StartsWith
            | TestOperation::EndsWith,
            Some(BatchValue::Column(_)) | None,
            _,
        )
        | (
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual
            | TestOperation::In
            | TestOperation::NotIn
            | TestOperation::NumericAtLeast
            | TestOperation::NaturalLess
            | TestOperation::Successor,
            _,
            _,
        ) => return None,
    };
    Some(TextTestKey {
        test: test.test,
        against,
    })
}

/// What a test against constants says of one text: the operators crate's meaning, asked for this one text. A glob test's
/// globs are compiled the first time the test is asked.
///
/// # Errors
///
/// The code names an expression it does not hold or that does not compile, or an affix test with no affix.
fn decided(
    means: &mut Deciding<'_, '_>,
    words: &FileWords,
    test: TextTest<'_>,
    whole: TextView<'_>,
) -> Result<RowKept, FilledError> {
    let wrong = FilledError::Code {
        named: MissingCode::Regex,
    };
    let globbed = |globs: &mut Option<TextGlobSet>, wanted: PredicateOutcome| {
        let compiled = globs.get_or_insert_with(|| {
            let texts: Vec<TextView<'_>> = match test.right {
                Some(BatchValue::Constant(glob)) => vec![TextView(text(words, glob).0)],
                Some(BatchValue::Column(_)) | None => test
                    .set
                    .iter()
                    .map(|glob| TextView(text(words, *glob).0))
                    .collect(),
            };
            globs_compiled(&texts, means.matcher)
        });
        kept(glob_compiled(whole, compiled, means.matcher), wanted)
    };
    let affixed = |asked: Test| match test.right {
        Some(BatchValue::Constant(affix)) => {
            let affix = text(words, affix);
            Ok(
                match AffixItem.holds(asked, &Datum::Text(whole), &Datum::Text(TextView(affix.0))) {
                    Ok(PredicateOutcome::Holds) => RowKept::Yes,
                    Ok(PredicateOutcome::Fails) | Err(_) => RowKept::No,
                },
            )
        }
        Some(BatchValue::Column(_)) | None => Err(FilledError::Code {
            named: MissingCode::Word,
        }),
    };
    match test.test {
        TestOperation::Matches(regex) => {
            let source = means
                .regexes
                .get(usize::try_from(regex.0).unwrap_or(usize::MAX))
                .ok_or(wrong.clone())?;
            let mut each = matches_each(
                std::iter::once(whole),
                TextView(&source.0),
                means.compiled,
                means.matcher,
            )
            .map_err(|_| wrong)?;
            Ok(match each.next() {
                Some(PredicateOutcome::Holds) => RowKept::Yes,
                Some(PredicateOutcome::Fails) | None => RowKept::No,
            })
        }
        TestOperation::Glob => Ok(globbed(means.globs, PredicateOutcome::Holds)),
        TestOperation::NotGlob => Ok(globbed(means.globs, PredicateOutcome::Fails)),
        TestOperation::StartsWith => affixed(Test::StartsWith),
        TestOperation::EndsWith => affixed(Test::EndsWith),
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => Err(FilledError::Code {
            named: MissingCode::Word,
        }),
    }
}

/// `Keep` of a test of a text against constants: each row's word is looked up in the test's verdicts, and a word not
/// decided yet is decided from its text, once. For a row the cost is the read of two bits.
///
/// # Errors
///
/// As `decided`, for the first word it faults on.
pub(crate) fn by_word(
    means: TextMeans<'_>,
    words: &FileWords,
    test: TextTest<'_>,
    key: TextTestKey,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) -> Result<(), FilledError> {
    let TextMeans {
        matcher,
        regexes,
        compiled,
        verdicts,
        stamp,
    } = means;
    let at = opened(verdicts, key, stamp);
    let Some(held) = verdicts.tests.get_mut(at.0) else {
        return Err(FilledError::Code {
            named: MissingCode::Word,
        });
    };
    let bounds = WordBounds {
        base: base_count(words),
        end: words_end(words),
    };
    let mut globs = held.globs.take();
    let mut fault = None;
    {
        let mut deciding = Deciding {
            matcher,
            regexes,
            compiled,
            globs: &mut globs,
        };
        selected(
            rows,
            keep,
            (0..rows.0).map(|row| {
                let word = value_at(test.left, Count(row));
                match known(held, word, bounds) {
                    WordKnown::Holds => RowKept::Yes,
                    WordKnown::Fails => RowKept::No,
                    WordKnown::NotYet => {
                        match decided(&mut deciding, words, test, TextView(text(words, word).0)) {
                            Ok(said) => {
                                remembered(held, word, bounds, said);
                                said
                            }
                            Err(found) => {
                                fault = Some(found);
                                RowKept::No
                            }
                        }
                    }
                }
            }),
        );
    }
    held.globs = globs;
    match fault {
        Some(found) => Err(found),
        None => Ok(()),
    }
}
