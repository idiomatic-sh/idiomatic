//! The `fillers` service's cases. The fact source answers each relation of the file's one request: a relation its language
//! fills with its rows, a relation the language has not read as refused for that reason, a node-table relation as refused (no
//! language fills it), and a filler that read the file in part as partial; a declared key's rows are found by the table's
//! index, and one relation refused leaves another of the same request written. The growth harness holds the fixture
//! language's fillers to what the catalog declares of each relation, and rejects each weaker filler: a row per pair, more
//! than one row for a file, a relation with no row, a language that lends no file.

use super::service::file_source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::fact_tables::reading::service::{every_row, rows_of};
use crate::domain::fact_tables::service::{answer_of, begun, request_of, table_of};
use crate::domain::fact_tables::types::{FactKey, FactSet};
use crate::domain::facts::types::{CellWord, FactAnswer, FileFacts};
use crate::domain::fillers::fixtures::growth::{GrowthFault, grown};
use crate::domain::fillers::fixtures::{PAIRS, TWICE, Walking, WalkingFile, weakness};
use crate::domain::fillers::types::Fillers as _;
use crate::domain::fixed_names::service::checked;
use crate::domain::node_table::service::{
    builder, diagnose, enter_node, finished, leave_node, node_count, token,
};
use crate::domain::node_table::types::{FieldCode, NodeTable};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    Column, Count, FileId, KindBits, Offset, Position, Relation,
};
use ryo_relations::domain::terms::types::NameText;

/// What one fill of a row asks for.
/// Which rows of the relation a case reads.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Every row.
    Whole,
    /// The links from the word at this place.
    Source(u32),
    /// The links to the word at this place.
    Target(u32),
}

/// A case's file and what it asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The file of the text asked for one relation, read as the ask says.
    Fills(Relation, &'static str, Ask),
    /// The file asked for two relations in one request.
    Then(Relation, Relation, &'static str),
    /// A growth run.
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
    /// The relation's answer (0 answered, 1 partial, 2 refused) and the rows read.
    Filled([usize; 2]),
    /// Each of two relations' answer and rows.
    Both([usize; 4]),
    /// A growth run with no fault: the elements of the two files, the rows of `syntax-error`, `language-target` and
    /// `token-link` at each.
    Grown([usize; 2], [[usize; 2]; 3]),
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
        name: CaseName("a_filled_relation_answers_its_rows"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxError, "one!", Ask::Whole),
        expect: Expect::Value(Seen::Filled([0, 1])),
    },
    Case {
        name: CaseName("a_relation_the_language_has_not_read_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::Function, "one", Ask::Whole),
        expect: Expect::Value(Seen::Filled([2, 0])),
    },
    Case {
        name: CaseName("a_node_table_relation_is_refused_no_language_fills_it"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxTree, "one", Ask::Whole),
        expect: Expect::Value(Seen::Filled([2, 0])),
    },
    Case {
        name: CaseName("a_filler_that_read_the_file_in_part_answers_partial"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::SyntaxError, "one?", Ask::Whole),
        expect: Expect::Value(Seen::Filled([1, 0])),
    },
    Case {
        name: CaseName("a_declared_keys_rows_are_the_links_from_that_word_alone"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", Ask::Source(1)),
        expect: Expect::Value(Seen::Filled([0, 1])),
    },
    Case {
        name: CaseName("a_second_declared_keys_rows_are_the_links_to_that_word_alone"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", Ask::Target(1)),
        expect: Expect::Value(Seen::Filled([0, 1])),
    },
    Case {
        name: CaseName("the_last_word_has_no_link_from_it"),
        word: Word::Succeeds,
        input: || Scenario::Fills(Relation::TokenLink, "one two three four", Ask::Source(3)),
        expect: Expect::Value(Seen::Filled([0, 0])),
    },
    Case {
        name: CaseName("a_refused_relation_leaves_another_of_the_same_request_written"),
        word: Word::Succeeds,
        input: || {
            Scenario::Then(
                Relation::TokenLink,
                Relation::Function,
                "one two three four",
            )
        },
        expect: Expect::Value(Seen::Both([0, 3, 2, 0])),
    },
    Case {
        name: CaseName("a_relations_rows_grow_with_the_file_as_its_catalog_item_declares"),
        word: Word::Succeeds,
        input: || Scenario::Grows(Lends::Sound),
        expect: Expect::Value(Seen::Grown([64, 256], [[1, 1], [1, 1], [62, 254]])),
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
        path: crate::domain::files::types::SourcePath("a.walk"),
        facts: &mut source,
    });
}

/// The text of a file of `elements` elements whose last word is `last`: the sentence and a word for every other element.
fn text_of(elements: Count, last: &str) -> String {
    let mut text = "w ".repeat(elements.0.saturating_sub(2));
    text.push_str(last);
    text
}

/// The kind of a relation's answer: 0 answered, 1 partial, 2 refused.
fn kind_of(answer: &FactAnswer) -> usize {
    match answer {
        FactAnswer::Answered => 0,
        FactAnswer::Partial(_) => 1,
        FactAnswer::Refused(_) => 2,
    }
}

/// The rows of a relation in the set, as `ask` reads them, in a file whose words start at the id `first`.
fn read_rows(set: &FactSet, relation: Relation, ask: Ask, first: u32) -> usize {
    let table = table_of(set, relation);
    let word = |place: u32| {
        FactKey::Cell(CellWord(
            node_id(FileId(1), Position(first.saturating_add(place))).0,
        ))
    };
    match ask {
        Ask::Whole => usize::try_from(every_row(table).end.0).unwrap_or(usize::MAX),
        Ask::Source(place) => rows_of(table, Column::TokenLink(TokenLinkColumn::Source), word(place))
            .into_iter()
            .count(),
        Ask::Target(place) => rows_of(table, Column::TokenLink(TokenLinkColumn::Target), word(place))
            .into_iter()
            .count(),
    }
}

/// The rows of a relation at the two sizes of a growth run.
fn pair(small: Option<Count>, large: Option<Count>) -> [usize; 2] {
    [
        small.map_or(0, |rows| rows.0),
        large.map_or(0, |rows| rows.0),
    ]
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let names = Walking::vocabulary().fixed;
    let fixed = checked(&names.0).expect("the fixture's fixed names");
    let mut seen = None;
    match scenario {
        Scenario::Fills(relation, text, ask) => lent(text, &mut |file| {
            let first = u32::try_from(node_count(file.table).0).unwrap_or(u32::MAX);
            let (mut words, mut set) = (file_words(&base_words(fixed, &[])), FactSet::default());
            let request = request_of(&[relation]);
            begun(&mut set, &request);
            file.facts.facts(&request, &mut words, &mut set);
            seen = Some(Seen::Filled([
                kind_of(&answer_of(&set, relation)),
                read_rows(&set, relation, ask, first),
            ]));
        }),
        Scenario::Then(first, second, text) => lent(text, &mut |file| {
            let (mut words, mut set) = (file_words(&base_words(fixed, &[])), FactSet::default());
            let request = request_of(&[first, second]);
            begun(&mut set, &request);
            file.facts.facts(&request, &mut words, &mut set);
            seen = Some(Seen::Both([
                kind_of(&answer_of(&set, first)),
                read_rows(&set, first, Ask::Whole, 0),
                kind_of(&answer_of(&set, second)),
                read_rows(&set, second, Ask::Whole, 0),
            ]));
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
            seen = Some(Seen::Grown(
                [report.small.0, report.large.0],
                [
                    rows(Relation::SyntaxError),
                    rows(Relation::LanguageTarget),
                    rows(Relation::TokenLink),
                ],
            ));
        }
    }
    Ok(seen.unwrap_or(Seen::Filled([usize::MAX; 2])))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
