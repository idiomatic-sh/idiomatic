//! The cases of a text test decided once for each word, each run through `made` on a pack the contract's encoding
//! wrote. A `Keep` of a text against constants reads a word's verdict where it was decided before: for the worker's
//! life when the word is the run's, for the file when it is the file's own, since the next file's own words are other
//! texts under the same numbers. A case makes its table for one file and then for a second whose text differs, and says
//! the texts kept in each.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::MadeTable;
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, keep_equal, node_read, of, operands, pipeline, read, rows, scan, scan_nodes,
    word,
};
use crate::domain::made_tables::fixtures::{Shown, Telling, framed, shown_as, world_with};
use crate::domain::made_tables::sources::service::{made, next_file};
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{
    MadeColumn, MadeTableId, MatchColumns, ReadColumn, RegexId, Register, StepOperand, StepRows,
    TestOperation,
};
use ryo_language::domain::facts::service::clear_words;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use std::borrow::Cow;

/// The keeps a case's tables make over the three tokens' texts, and the two files' texts.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A regular expression (`^[ac]`) over two files whose own words are other texts under the same numbers: `xb`
    /// and `cd`, then `ab` and `zd`.
    OwnWords,
    /// A glob that is the text `ef`, a word of the run, over the same two files.
    RunWord,
    /// Two tables in one file, each a glob keep against another constant (`a*`, then `?d`).
    TwoTests,
    /// A keep against a set of two globs, then, in the second file, the same.
    SetOfGlobs,
    /// A keep of a prefix (`a`) over the two files.
    Prefix,
    /// A keep of the relation's rows whose first column, a text, matches the glob the row's second column is: nothing
    /// of it is the same from row to row.
    OwnGlob,
    /// The rows the same glob keeps out.
    OwnGlobNegated,
    /// The relation's rows whose first text is before its second in the natural order of texts.
    NaturalOrder,
    /// A glob (`a*`) of THE FILE, read from a made table's own file column, over two files of one worker whose paths
    /// are `ab/cd.walk` and `zz/cd.walk`: the tokens' texts are kept in the file whose path matches.
    FilePath,
    /// The same of the file read from the node table's file column.
    NodeFilePath,
}

type Input = Asked;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_verdict_of_a_files_own_word_is_not_carried_to_the_next_files_word_of_that_number",
        ),
        word: Word::Succeeds,
        input: || Asked::OwnWords,
        expect: Expect::Value(Cow::Borrowed("xb cd ef: \"cd\" | ab zd ef: \"ab\"")),
    },
    Case {
        name: CaseName("a_verdict_of_a_word_of_the_run_holds_in_every_file"),
        word: Word::Succeeds,
        input: || Asked::RunWord,
        expect: Expect::Value(Cow::Borrowed("xb cd ef: \"ef\" | ab zd ef: \"ef\"")),
    },
    Case {
        name: CaseName("two_tests_against_different_constants_keep_their_own_verdicts"),
        word: Word::Succeeds,
        input: || Asked::TwoTests,
        expect: Expect::Value(Cow::Borrowed(
            "xb cd ef:  and \"cd\" | ab zd ef: \"ab\" and \"zd\"",
        )),
    },
    Case {
        name: CaseName("a_glob_keep_against_a_set_is_decided_by_any_of_its_globs_in_each_file"),
        word: Word::Succeeds,
        input: || Asked::SetOfGlobs,
        expect: Expect::Value(Cow::Borrowed("xb cd ef: \"cd\" | ab zd ef: \"ab\"; \"zd\"")),
    },
    Case {
        name: CaseName("a_prefix_keep_is_decided_by_each_files_own_texts"),
        word: Word::Succeeds,
        input: || Asked::Prefix,
        expect: Expect::Value(Cow::Borrowed("xb cd ef:  | ab zd ef: \"ab\"")),
    },
    Case {
        name: CaseName("a_rows_own_glob_is_read_for_its_row"),
        word: Word::Succeeds,
        input: || Asked::OwnGlob,
        expect: Expect::Value(Cow::Borrowed(
            "xb cd ef: \"ef\"; \"a\" | ab zd ef: \"ef\"; \"a\"",
        )),
    },
    Case {
        name: CaseName("a_rows_own_glob_negated_keeps_the_other_rows"),
        word: Word::Succeeds,
        input: || Asked::OwnGlobNegated,
        expect: Expect::Value(Cow::Borrowed(
            "xb cd ef: \"f\"; \"ef\" | ab zd ef: \"f\"; \"ef\"",
        )),
    },
    Case {
        name: CaseName("a_natural_order_keeps_the_rows_whose_first_text_is_before_the_second"),
        word: Word::Succeeds,
        input: || Asked::NaturalOrder,
        expect: Expect::Value(Cow::Borrowed("xb cd ef: \"a\" | ab zd ef: \"a\"")),
    },
    Case {
        name: CaseName("a_text_test_of_the_file_reads_each_files_own_path"),
        word: Word::Succeeds,
        input: || Asked::FilePath,
        expect: Expect::Value(Cow::Borrowed(
            "xb cd ef: \"first\"; \"first\"; \"first\" and \"xb\"; \"cd\"; \"ef\" | ab zd ef: \"first\"; \"first\"; \"first\" and ",
        )),
    },
    Case {
        name: CaseName("a_text_test_of_a_nodes_file_reads_the_files_path"),
        word: Word::Succeeds,
        input: || Asked::NodeFilePath,
        expect: Expect::Value(Cow::Borrowed(
            "xb cd ef: \"xb\"; \"cd\"; \"ef\" | ab zd ef: ",
        )),
    },
];

/// A table of the tokens' texts in the files whose path the glob `a*` matches: the file is read from the first table's
/// own file column when there is one, else from the node table's.
fn of_the_file(made: Option<MadeTableId>) -> MadeTable {
    let tree = Relation::SemanticSyntax;
    let mut all = Vec::from(scan_nodes(tree));
    all.push(keep_equal(1, 13));
    all.push(node_read(tree, NodeColumn::Text, 0, 2));
    all.push(match made {
        Some(table) => FillStage::Probe {
            source: StepRows::Made(table),
            columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(1))])),
            values: operands([of(2)]),
            range: None,
            mode: UseMode::Rows,
            out: Register(3),
        },
        None => node_read(tree, NodeColumn::Kind, 0, 3),
    });
    all.push(match made {
        Some(table) => FillStage::Read {
            source: StepRows::Made(table),
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(3),
            out: Register(4),
        },
        None => node_read(tree, NodeColumn::File, 0, 4),
    });
    all.push(FillStage::Keep {
        test: TestOperation::Glob,
        left: of(4),
        right: word(1),
    });
    rows(1, pipeline(all, [of(2)]))
}

/// A table of the file and each token's text: its first column is the file's own.
fn texts_with_the_file() -> MadeTable {
    let tree = Relation::SemanticSyntax;
    let mut all = Vec::from(scan_nodes(tree));
    all.push(keep_equal(1, 13));
    all.push(node_read(tree, NodeColumn::Text, 0, 2));
    rows(2, pipeline(all, [StepOperand::File, of(2)]))
}

/// A table of the tokens whose text the keep holds for.
fn kept(test: TestOperation, right: StepOperand) -> MadeTable {
    let tree = Relation::SemanticSyntax;
    let mut all = Vec::from(scan_nodes(tree));
    all.push(keep_equal(1, 13));
    all.push(node_read(tree, NodeColumn::Text, 0, 2));
    all.push(FillStage::Keep {
        test,
        left: of(2),
        right,
    });
    rows(1, pipeline(all, [of(2)]))
}

/// The relation's rows: a text's word, and a glob's word. `ef` against `ef`, `a` against `a*`, `f` against `?d`, `ef`
/// against `a*`.
const ROWS: [[u64; 2]; 4] = [[3, 3], [8, 1], [9, 2], [3, 1]];

/// A table of the relation's first column where the test holds between it and the row's second column.
fn own_glob(test: TestOperation) -> MadeTable {
    rows(
        1,
        pipeline(
            vec![
                scan(0),
                read(FIRST, 0, 1),
                read(SECOND, 0, 2),
                FillStage::Keep {
                    test,
                    left: of(1),
                    right: of(2),
                },
            ],
            [of(1)],
        ),
    )
}

/// The tables of a case.
fn tables(asked: Asked) -> Vec<MadeTable> {
    match asked {
        Asked::OwnWords => vec![kept(TestOperation::Matches(RegexId(0)), of(2))],
        Asked::RunWord => vec![kept(TestOperation::Glob, word(3))],
        Asked::TwoTests => vec![
            kept(TestOperation::Glob, word(1)),
            kept(TestOperation::Glob, word(2)),
        ],
        Asked::SetOfGlobs => vec![kept(
            TestOperation::Glob,
            StepOperand::Words(Box::new([CellWord(1), CellWord(2)])),
        )],
        Asked::Prefix => vec![kept(TestOperation::StartsWith, word(8))],
        Asked::OwnGlob => vec![own_glob(TestOperation::Glob)],
        Asked::OwnGlobNegated => vec![own_glob(TestOperation::NotGlob)],
        Asked::NaturalOrder => vec![own_glob(TestOperation::NaturalLess)],
        Asked::FilePath => vec![texts_with_the_file(), of_the_file(Some(MadeTableId(0)))],
        Asked::NodeFilePath => vec![of_the_file(None)],
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let made_tables = tables(asked);
    let count = u32::try_from(made_tables.len()).unwrap_or(0);
    let mut world = world_with(made_tables, Vec::new(), &ROWS, Telling::Keyed)?;
    let mut said = Vec::new();
    // The second file's tokens are other texts of the same lengths: its own words take the first file's numbers.
    for (text, path) in [("xb cd ef", "ab/cd.walk"), ("ab zd ef", "zz/cd.walk")] {
        world.text = text.as_bytes().to_vec();
        world.path = path;
        next_file(&mut world.store);
        clear_words(&mut world.words);
        let mut each = Vec::new();
        for table in 0..count {
            {
                let (mut frame, looped) = framed(&mut world);
                let _now = made(&mut frame, looped, MadeTableId(table))?;
            }
            each.push(shown_as(&world, MadeTableId(table), &[Shown::Text]));
        }
        said.push(format!("{text}: {}", each.join(" and ")));
    }
    Ok(Cow::Owned(said.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
