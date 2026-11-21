//! The cases of the size of a text test's verdicts, each run through `made` on a pack the contract's encoding wrote. A
//! test keeps two bits for each word of the run and for each own word of the file, and one verdict for every cell that
//! is no word: never a place for a cell's value. Each case counts what every test keeps after the fill, in words of
//! thirty-two verdicts, and says the most the file's words allow. The cells that are no words are about a million,
//! not an id with its file in the high half: a table sized by them is seen by the count and hurts nothing.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::MadeTable;
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, keep_equal, node_read, of, pipeline, probe, read, rows, scan, scan_nodes, word,
};
use crate::domain::made_tables::fixtures::{Telling, framed, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::stages::verdicts::service::kept_bits;
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{MadeTableId, RegexId, TestOperation};
use ryo_language::domain::facts::service::{base_count, words_end};
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use std::borrow::Cow;

/// The keep a case's table makes.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// After a probe that gives several rows for one: a negated glob of a column whose cells are numbers of about a
    /// million, no words.
    NotWordsAfterAFanOut,
    /// The same glob not negated: a cell that is no word is the empty text, which the glob does not match.
    NotWordsKeptOut,
    /// A regular expression over the three tokens' texts: one word of the run and two of the file's own.
    WordsOfTheFile,
}

type Input = Asked;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows: a key, and a number that is no word.
const ROWS: [[u64; 2]; 5] = [
    [1, 1_000_003],
    [2, 1_000_001],
    [1, 1_000_007],
    [3, 999_999],
    [1, 1_000_003],
];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("cells_that_are_no_words_are_given_no_place_whatever_their_value"),
        word: Word::Succeeds,
        input: || Asked::NotWordsAfterAFanOut,
        expect: Expect::Value(Cow::Borrowed(
            "11 rows kept | 1 test keeps verdicts: of the run 0 of at most 1, of the file 0 of at most 0",
        )),
    },
    Case {
        name: CaseName("a_tests_verdicts_are_no_longer_than_the_files_words"),
        word: Word::Succeeds,
        input: || Asked::WordsOfTheFile,
        expect: Expect::Value(Cow::Borrowed(
            "2 rows kept | 1 test keeps verdicts: of the run 1 of at most 1, of the file 1 of at most 1",
        )),
    },
    Case {
        name: CaseName("a_cell_that_is_no_word_is_tested_as_the_empty_text"),
        word: Word::Succeeds,
        input: || Asked::NotWordsKeptOut,
        expect: Expect::Value(Cow::Borrowed(
            "0 rows kept | 1 test keeps verdicts: of the run 0 of at most 1, of the file 0 of at most 0",
        )),
    },
];

/// The table of a case.
fn table_of_case(asked: Asked) -> MadeTable {
    match asked {
        Asked::NotWordsAfterAFanOut | Asked::NotWordsKeptOut => rows(
            1,
            pipeline(
                vec![
                    scan(0),
                    read(FIRST, 0, 1),
                    probe(1, None, UseMode::Rows, 2),
                    read(SECOND, 2, 3),
                    FillStage::Keep {
                        test: if matches!(asked, Asked::NotWordsAfterAFanOut) {
                            TestOperation::NotGlob
                        } else {
                            TestOperation::Glob
                        },
                        left: of(3),
                        right: word(1),
                    },
                ],
                [of(3)],
            ),
        ),
        Asked::WordsOfTheFile => {
            let tree = Relation::SemanticSyntax;
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 13));
            all.push(node_read(tree, NodeColumn::Text, 0, 2));
            all.push(FillStage::Keep {
                test: TestOperation::Matches(RegexId(0)),
                left: of(2),
                right: of(2),
            });
            rows(1, pipeline(all, [of(2)]))
        }
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let mut world = world_with(
        vec![table_of_case(asked)],
        Vec::new(),
        &ROWS,
        Telling::Keyed,
    )?;
    {
        let (mut frame, looped) = framed(&mut world);
        let _now = made(&mut frame, looped, MadeTableId(0))?;
    }
    let made_rows = table_of(&world.store, MadeTableId(0)).map_or(0, |slot| slot.rows.count.0);
    // The most a test may keep: a word of thirty-two verdicts for every thirty-two words, of the run and of the file.
    let base = usize::try_from(base_count(&world.words).0).unwrap_or(usize::MAX);
    let end = usize::try_from(words_end(&world.words).0).unwrap_or(usize::MAX);
    let bound = [base.div_ceil(32), end.saturating_sub(base).div_ceil(32)];
    let kept: Vec<String> = world
        .store
        .verdicts
        .tests
        .iter()
        .map(|test| {
            let [run_words, own_words] = kept_bits(test);
            format!(
                "of the run {} of at most {}, of the file {} of at most {}",
                run_words.0,
                bound.first().copied().unwrap_or(0),
                own_words.0,
                bound.last().copied().unwrap_or(0)
            )
        })
        .collect();
    Ok(Cow::Owned(format!(
        "{made_rows} rows kept | {} test keeps verdicts: {}",
        kept.len(),
        kept.join("; ")
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
