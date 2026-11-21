//! The `filled` service's cases: a key is asked of the language alone and once; a whole fill after keys starts past their
//! rows and holds each row once; a language that answers every row to a key has made the relation whole; a refusal is
//! asked once; another file's rows are gone.

use super::errors::FilledError;
use super::service::{keyed, rows_held, whole};
use super::types::{AskedRows, FilledRelations, FilledState, RelationAsk};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::RelationNumber;
use crate::domain::indexes::types::FileStamp;
use ryo_language::domain::facts::service::{base_words, file_words};
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FillExtent, FilledColumn, FilledColumns, Wanted,
};
use ryo_language::domain::fixed_names::service::checked;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::types::{Column, Count, Relation};
use std::borrow::Cow;

/// How the language of a case answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    /// A key's rows alone to a key.
    Keyed,
    /// Every row to a key, saying so.
    WholeToAKey,
    /// A refusal.
    Refusing,
}

/// One read of a case.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// The rows whose source is the cell.
    Source(u64),
    /// The rows whose target is the cell.
    Target(u64),
    /// Every row.
    Whole,
    /// The next file starts.
    NextFile,
}

/// What a case reads, of which language.
#[derive(Clone, Copy, Debug)]
struct Asked {
    /// The language.
    language: Language,
    /// The reads, in order.
    reads: &'static [Read],
}

type Input = Asked;
/// Per read: the first row, the row past the last, the rows the language wrote, 1 when the relation is whole after it;
/// then 99, how many fills the language was asked, how many of them for a key, and the rows the relation holds.
type Value = Cow<'static, [u64]>;
type Failure = FilledError;

/// The relation's rows as the language holds them: source and target.
const ROWS: [(u64, u64); 5] = [(1, 10), (2, 20), (1, 30), (3, 10), (1, 40)];

/// A language that counts what it is asked.
struct Told {
    /// How it answers.
    language: Language,
    /// What it was asked, in order.
    asked: Vec<Wanted>,
}

impl FactSource for Told {
    fn fill(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        _: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer {
        self.asked.push(wanted);
        if self.language == Language::Refusing {
            return FactAnswer::Refused(AbsenceCause::NotYetRead);
        }
        assert_eq!(relation, Relation::TokenLink);
        if out.0.len() < 5 {
            out.0.resize_with(5, FilledColumn::default);
        }
        let (wanted, extent) = match (wanted, self.language) {
            (Wanted::Key(..), Language::WholeToAKey) => (Wanted::Whole, FillExtent::Whole),
            (Wanted::Key(..) | Wanted::Whole, _) => (wanted, FillExtent::Asked),
        };
        for (source, target) in ROWS {
            let held = match wanted {
                Wanted::Whole => true,
                Wanted::Key(Column::TokenLink(TokenLinkColumn::Source), cell) => cell.0 == source,
                Wanted::Key(Column::TokenLink(TokenLinkColumn::Target), cell) => cell.0 == target,
                Wanted::Key(..) => false,
            };
            if held {
                for (column, cell) in out.0.iter_mut().zip([source, target, 7, 0, 0]) {
                    column.0.push(CellWord(cell));
                }
            }
        }
        FactAnswer::Answered(extent)
    }

    fn render(&mut self, _: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_read_by_key_asks_the_language_for_that_key_alone_and_once"),
        word: Word::Succeeds,
        input: || Asked {
            language: Language::Keyed,
            reads: &[Read::Source(1), Read::Source(1), Read::Target(10)],
        },
        expect: Expect::Value(Cow::Borrowed(&[
            0, 3, 3, 0, 0, 3, 0, 0, 3, 5, 2, 0, 99, 2, 2, 5,
        ])),
    },
    Case {
        name: CaseName(
            "a_whole_fill_after_keyed_fills_starts_past_their_rows_and_holds_each_row_once",
        ),
        word: Word::Succeeds,
        input: || Asked {
            language: Language::Keyed,
            reads: &[Read::Source(2), Read::Whole, Read::Whole, Read::Source(3)],
        },
        expect: Expect::Value(Cow::Borrowed(&[
            0, 1, 1, 0, 1, 6, 5, 1, 1, 6, 0, 1, 1, 6, 0, 1, 99, 2, 1, 6,
        ])),
    },
    Case {
        name: CaseName("a_language_that_answers_every_row_to_a_key_has_made_the_relation_whole"),
        word: Word::Succeeds,
        input: || Asked {
            language: Language::WholeToAKey,
            reads: &[Read::Source(1), Read::Source(2), Read::Whole],
        },
        expect: Expect::Value(Cow::Borrowed(&[
            0, 5, 5, 1, 0, 5, 0, 1, 0, 5, 0, 1, 99, 1, 1, 5,
        ])),
    },
    Case {
        name: CaseName("the_next_file_holds_none_of_the_last_files_rows"),
        word: Word::Succeeds,
        input: || Asked {
            language: Language::Keyed,
            reads: &[Read::Whole, Read::NextFile, Read::Source(3)],
        },
        expect: Expect::Value(Cow::Borrowed(&[0, 5, 5, 1, 0, 1, 1, 0, 99, 2, 1, 1])),
    },
    Case {
        name: CaseName("a_refused_relation_is_asked_once_and_refused_to_every_read"),
        word: Word::Refuses,
        input: || Asked {
            language: Language::Refusing,
            reads: &[Read::Whole],
        },
        expect: Expect::Error(FilledError::Refused {
            relation: Relation::TokenLink,
            cause: AbsenceCause::NotYetRead,
        }),
    },
];

/// One read of the relation through the service.
fn read(
    relations: &mut FilledRelations,
    told: &mut Told,
    words: &mut FileWords,
    stamp: FileStamp,
    read: Read,
) -> Result<AskedRows, FilledError> {
    let ask = RelationAsk {
        number: RelationNumber(2),
        relation: Relation::TokenLink,
        stamp,
        facts: told,
        words,
    };
    match read {
        Read::Source(cell) => keyed(relations, ask, Count(0), CellWord(cell)),
        Read::Target(cell) => keyed(relations, ask, Count(1), CellWord(cell)),
        Read::Whole | Read::NextFile => whole(relations, ask),
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let mut told = Told {
        language: asked.language,
        asked: Vec::new(),
    };
    let mut words = file_words(&base_words(
        checked(&[]).expect("no fixed name to refuse"),
        &[],
    ));
    let mut relations = FilledRelations::default();
    let mut stamp = FileStamp(1);
    let mut said = Vec::new();
    for each in asked.reads {
        if matches!(each, Read::NextFile) {
            stamp = FileStamp(2);
            continue;
        }
        let first = read(&mut relations, &mut told, &mut words, stamp, *each);
        if asked.language == Language::Refusing {
            // The second read is refused without the language being asked again.
            let again = read(&mut relations, &mut told, &mut words, stamp, *each);
            assert_eq!(first, again);
            assert_eq!(told.asked.len(), 1);
        }
        let rows = first?;
        said.extend([
            rows.rows.start.0 as u64,
            rows.rows.end.0 as u64,
            rows.filled.0 as u64,
            u64::from(rows.state == FilledState::Whole),
        ]);
    }
    let keys = told
        .asked
        .iter()
        .filter(|wanted| matches!(wanted, Wanted::Key(..)))
        .count();
    said.extend([
        99,
        told.asked.len() as u64,
        keys as u64,
        rows_held(&relations, RelationNumber(2), stamp).0 as u64,
    ]);
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
