//! The `fillers` service's cases. The fact source answers a relation its language fills with one column per column and its rows,
//! a relation the language has not read as refused for that reason, a node-table relation as refused (no language fills it), and
//! a filler that read the file in part as partial. A filler with no body for a key writes every row and says so; a filler with a
//! body for a declared key writes that key's rows alone; fills append, and an absent relation leaves the rows before it. The
//! growth harness holds the fixture language's fillers to what the catalog declares of each relation, and rejects each weaker
//! filler: a row per pair, more than one row for a file, a key with no body, a key body that writes every row, a relation with
//! no row, a language that lends no file.

use super::service::file_source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{
    CellWord, FactAnswer, FileFacts, FillExtent, FilledColumns, Wanted,
};
use crate::domain::fillers::fixtures::growth::{GrowthFault, grown};
use crate::domain::fillers::fixtures::{
    PAIRS, TWICE, UNKEYED, WHOLE_BY_KEY, Walking, WalkingFile, weakness,
};
use crate::domain::fillers::types::Fillers as _;
use crate::domain::node_table::service::{
    builder, diagnose, enter_node, finished, leave_node, node_count, token,
};
use crate::domain::node_table::types::{FieldCode, NodeTable};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorColumn;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    Column, Count, FileId, KindBits, Offset, Position, Relation,
};
use ryo_relations::domain::terms::types::NameText;

/// What one fill of a row asks for.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Every row.
    Whole,
    /// The links from the word of this place among the file's words.
    Source(u32),
    /// The links to the word of this place.
    Target(u32),
    /// The links of one kind: a column `token-link` declares no key for.
    LinkKind,
    /// The report of one id: a column of `syntax-error`, which declares no key.
    ErrorId,
}

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Fills a relation of a file of the text, once for each ask, into the same columns.
    Fills(Relation, &'static str, &'static [Ask]),
    /// Fills the first relation whole, then asks the second into the same columns.
    Then(Relation, Relation, &'static str),
    /// Runs the growth harness over what a language lends.
    Grows(Lends),
}

/// What a language lends the growth harness.
#[derive(Clone, Copy, Debug)]
enum Lends {
    /// Files of words that end in a word with a diagnostic: every filler as the catalog declares.
    Sound,
    /// The same files with one of the fixture's bytes, which makes one filler weaker than the catalog declares.
    Weak(u8),
    /// Files of words with no diagnostic: `syntax-error` has no row.
    NoError,
    /// No file.
    Nothing,
}

/// What a row answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seen {
    /// The last fill's answer (0 answered as asked, 1 partial, 2 refused, 3 answered with every row to a key), the columns
    /// written and the rows of the first.
    Filled([usize; 3]),
    /// A growth run with no fault: the elements of the two files, the rows of `syntax-error`, `language-target` and
    /// `token-link` at each, and the rows of one key of `token-link`'s source and of its target at each.
    Grown([usize; 2], [[usize; 2]; 3], [[usize; 2]; 2]),
}

type Input = Scenario;
type Value = Seen;
/// The first fault a growth run found.
type Failure = GrowthFault;

/// The elements of the smaller file of a growth run.
const ELEMENTS: Count = Count(64);

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filled_relation_answers_its_rows_column_by_column"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxError, "one!", &[Ask::Whole]),
        expect: Expect::Value(Seen::Filled([0, 7, 1])),
    },
    Case {
        name: CaseName("a_relation_the_language_has_not_read_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::Function, "one", &[Ask::Whole]),
        expect: Expect::Value(Seen::Filled([2, 0, 0])),
    },
    Case {
        name: CaseName("a_node_table_relation_is_refused_no_language_fills_it"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxTree, "one", &[Ask::Whole]),
        expect: Expect::Value(Seen::Filled([2, 0, 0])),
    },
    Case {
        name: CaseName("a_filler_that_read_the_file_in_part_answers_partial"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxError, "one?", &[Ask::Whole]),
        expect: Expect::Value(Seen::Filled([1, 7, 0])),
    },
    Case {
        name: CaseName("a_filler_with_no_body_for_a_key_writes_every_row_and_says_so"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxError, "one! two!", &[Ask::ErrorId]),
        expect: Expect::Value(Seen::Filled([3, 7, 2])),
    },
    Case {
        name: CaseName("a_fill_of_a_declared_key_writes_the_links_from_that_word_alone"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", &[Ask::Source(1)]),
        expect: Expect::Value(Seen::Filled([0, 5, 1])),
    },
    Case {
        name: CaseName("a_fill_of_a_second_declared_key_writes_the_links_to_that_word_alone"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", &[Ask::Target(1)]),
        expect: Expect::Value(Seen::Filled([0, 5, 1])),
    },
    Case {
        name: CaseName("a_key_of_the_last_word_has_no_link_and_is_answered_as_asked"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", &[Ask::Source(3)]),
        expect: Expect::Value(Seen::Filled([0, 5, 0])),
    },
    Case {
        name: CaseName("a_key_on_a_column_with_no_body_writes_every_row_and_says_so"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", &[Ask::LinkKind]),
        expect: Expect::Value(Seen::Filled([3, 5, 3])),
    },
    Case {
        name: CaseName("fills_append_a_whole_fill_goes_after_the_rows_of_two_keys"),
        word: Word::Succeeds,
        input: || {
            Scenario::Fills(
                Relation::TokenLink,
                "one two three four",
                &[Ask::Source(0), Ask::Target(3), Ask::Whole],
            )
        },
        expect: Expect::Value(Seen::Filled([0, 5, 5])),
    },
    Case {
        name: CaseName("an_absent_relation_leaves_the_rows_written_before_it"),
        word: Word::Succeeds,
        input: || {
            Scenario::Then(
                Relation::TokenLink,
                Relation::Function,
                "one two three four",
            )
        },
        expect: Expect::Value(Seen::Filled([2, 5, 3])),
    },
    Case {
        name: CaseName("a_relations_rows_grow_with_the_file_as_its_catalog_item_declares"),
        word: Word::Succeeds,
        input: || Scenario::Grows(Lends::Sound),
        expect: Expect::Value(Seen::Grown(
            [64, 256],
            [[1, 1], [1, 1], [62, 254]],
            [[1, 1], [1, 1]],
        )),
    },
    Case {
        name: CaseName("a_relation_that_writes_a_row_per_pair_declared_per_element_is_rejected"),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::Weak(PAIRS)),
        expect: Expect::Error(GrowthFault::Grew(Relation::TokenLink)),
    },
    Case {
        name: CaseName("a_relation_that_writes_two_rows_for_a_file_declared_per_file_is_rejected"),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::Weak(TWICE)),
        expect: Expect::Error(GrowthFault::Grew(Relation::LanguageTarget)),
    },
    Case {
        name: CaseName(
            "a_fill_of_a_declared_key_that_writes_the_whole_relation_and_says_so_is_rejected",
        ),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::Weak(UNKEYED)),
        expect: Expect::Error(GrowthFault::KeyNotAsked(Column::TokenLink(
            TokenLinkColumn::Source,
        ))),
    },
    Case {
        name: CaseName("a_fill_of_a_declared_key_that_writes_other_rows_than_the_keys_is_rejected"),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::Weak(WHOLE_BY_KEY)),
        expect: Expect::Error(GrowthFault::KeyRows(Column::TokenLink(
            TokenLinkColumn::Source,
        ))),
    },
    Case {
        name: CaseName("a_relation_with_no_row_for_the_larger_file_is_rejected_as_unproven"),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::NoError),
        expect: Expect::Error(GrowthFault::Unproven(Relation::SyntaxError)),
    },
    Case {
        name: CaseName("a_language_that_lends_no_file_is_rejected"),
        word: Word::Rejects,
        input: || Scenario::Grows(Lends::Nothing),
        expect: Expect::Error(GrowthFault::Sources),
    },
];

/// Where a byte is in a file.
fn offset(at: usize) -> Offset {
    Offset(u32::try_from(at).unwrap_or(u32::MAX))
}

/// Lends the file of `text` as the fixture language's walk writes its own tree: a sentence over one word for each word of the
/// text, and a diagnostic at each `!`.
fn lent(text: &str, visit: &mut dyn FnMut(FileFacts<'_>)) {
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(4));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    let mut from: usize = 0;
    for word in text.split(' ').filter(|word| !word.is_empty()) {
        let to = from.saturating_add(word.len());
        token(
            &mut writer,
            KindBits(2),
            FieldCode(0),
            offset(from),
            offset(to),
        );
        if word.ends_with('!') {
            diagnose(
                &mut writer,
                offset(to.saturating_sub(1)),
                offset(to),
                NameText("an exclamation"),
            );
        }
        from = to.saturating_add(1);
    }
    leave_node(&mut writer, offset(text.len()));
    let table = finished(&mut writer);
    let mut file = WalkingFile {
        table,
        source: text.as_bytes(),
        file: FileId(1),
        fills: Count(0),
        weakness: weakness(text.as_bytes()),
    };
    let mut source = file_source::<Walking>(&mut file);
    visit(FileFacts {
        table,
        source: ByteSlice(text.as_bytes()),
        file: FileId(1),
        facts: &mut source,
    });
}

/// The text of a file of `elements` elements whose last word is `last`: the sentence and a word for every other element.
fn text_of(elements: Count, last: &str) -> String {
    let mut text = "w ".repeat(elements.0.saturating_sub(2));
    text.push_str(last);
    text
}

/// The cell a fill is asked by, in a file whose words start at the id `first`.
fn wanted(ask: Ask, first: u32) -> Wanted {
    let word = |place: u32| CellWord(node_id(FileId(1), Position(first.saturating_add(place))).0);
    match ask {
        Ask::Whole => Wanted::Whole,
        Ask::Source(place) => Wanted::Key(Column::TokenLink(TokenLinkColumn::Source), word(place)),
        Ask::Target(place) => Wanted::Key(Column::TokenLink(TokenLinkColumn::Target), word(place)),
        Ask::LinkKind => Wanted::Key(Column::TokenLink(TokenLinkColumn::LinkKind), CellWord(1)),
        Ask::ErrorId => Wanted::Key(Column::SyntaxError(SyntaxErrorColumn::Id), CellWord(0)),
    }
}

/// The answer's kind, the columns written and the rows of the first.
fn filled(answer: &FactAnswer, columns: &FilledColumns) -> Seen {
    let kind = match answer {
        FactAnswer::Answered(FillExtent::Asked) => 0,
        FactAnswer::Partial(_) => 1,
        FactAnswer::Refused(_) => 2,
        FactAnswer::Answered(FillExtent::Whole) => 3,
    };
    Seen::Filled([
        kind,
        columns.0.len(),
        columns.0.first().map_or(0, |column| column.0.len()),
    ])
}

/// The rows of a relation, or of a key column, at the two sizes of a growth run.
fn pair(small: Option<Count>, large: Option<Count>) -> [usize; 2] {
    [
        small.map_or(0, |rows| rows.0),
        large.map_or(0, |rows| rows.0),
    ]
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let names = Walking::vocabulary().fixed;
    let mut seen = None;
    match scenario {
        Scenario::Fills(relation, text, asks) => lent(text, &mut |file| {
            let first = u32::try_from(node_count(file.table).0).unwrap_or(u32::MAX);
            let (mut words, mut columns) = (
                file_words(&base_words(&names, &[])),
                FilledColumns::default(),
            );
            for ask in asks {
                let answer =
                    file.facts
                        .fill(relation, wanted(*ask, first), &mut words, &mut columns);
                seen = Some(filled(&answer, &columns));
            }
        }),
        Scenario::Then(first, second, text) => lent(text, &mut |file| {
            let (mut words, mut columns) = (
                file_words(&base_words(&names, &[])),
                FilledColumns::default(),
            );
            let _written = file
                .facts
                .fill(first, Wanted::Whole, &mut words, &mut columns);
            let answer = file
                .facts
                .fill(second, Wanted::Whole, &mut words, &mut columns);
            seen = Some(filled(&answer, &columns));
        }),
        Scenario::Grows(lends) => {
            let last = match lends {
                Lends::Sound => Some("end!".to_owned()),
                Lends::Weak(byte) => Some(format!("{}!", char::from(byte))),
                Lends::NoError => Some("end".to_owned()),
                Lends::Nothing => None,
            };
            let report = grown(&names, ELEMENTS, &mut |elements, visit| {
                if let Some(last) = &last {
                    lent(&text_of(elements, last), visit);
                }
            });
            if let Some(fault) = report.faults.first() {
                return Err(*fault);
            }
            let rows = |relation: Relation| {
                let found = report
                    .relations
                    .iter()
                    .find(|each| each.relation == relation);
                pair(found.map(|each| each.small), found.map(|each| each.large))
            };
            let key = |column: TokenLinkColumn| {
                let found = report
                    .keys
                    .iter()
                    .find(|each| each.column == Column::TokenLink(column));
                pair(found.map(|each| each.small), found.map(|each| each.large))
            };
            seen = Some(Seen::Grown(
                [report.small.0, report.large.0],
                [
                    rows(Relation::SyntaxError),
                    rows(Relation::LanguageTarget),
                    rows(Relation::TokenLink),
                ],
                [key(TokenLinkColumn::Source), key(TokenLinkColumn::Target)],
            ));
        }
    }
    Ok(seen.unwrap_or(Seen::Filled([usize::MAX; 3])))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
