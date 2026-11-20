//! What the `walk` use case promises: a language walks a file into the node table and lends it, with its number in the run and
//! its fillers, to the visit; the language that reads nothing refuses every file, and a parser that fails is the language's fault.
//! A held language names its kinds, fields and fixed words by place: a field's code in the table is the place of its name, an
//! embedded segment follows the file's own tree, and the language that reads nothing names nothing.

use super::errors::WalkError;
use super::fixtures::FAILING;
use super::interface::WalkUseCase as _;
use super::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use super::use_case::WalkUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{
    FactAnswer, FileFacts, FillExtent, FilledColumns, FixedNames, Wanted,
};
use crate::domain::files::types::SourcePath;
use crate::domain::fillers::fixtures::Walking;
use crate::domain::node_table::service::{
    builder, element_count, embedded_segments, field_of, node_count, own_end,
};
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId, Position, Relation};

/// What a row walks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A file of words, through the fixture language.
    Words(&'static str),
    /// A file the fixture language's parser fails on.
    Failing,
    /// Any file, through the language that reads nothing.
    Nothing,
    /// A file of one word, then a file of words, through the fixture language held behind `WalkEngine` with one worker's buffers.
    Held(&'static str),
    /// Any file, through the language that reads nothing held behind `WalkEngine`.
    HeldNothing,
    /// The names the fixture language states for the run, held behind `WalkEngine`.
    Names,
    /// The names the language that reads nothing states.
    NamesOfNothing,
    /// A file of words, for the name of the field its first word fills.
    Field(&'static str),
    /// A file of words, for where its own tree ends and its embedded segments are.
    Embedded(&'static str),
}

/// What the visit saw of the lent file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    /// Which file of the run the lent file says it is.
    file: FileId,
    /// The file's syntax nodes.
    nodes: Count,
    /// Its nodes and tokens.
    elements: Count,
    /// The `syntax-error` rows its filler wrote.
    syntax_errors: Count,
    /// Whether the filler answered every row.
    answer: FactAnswer,
}

/// What a row answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    /// What the visit saw of the lent file; nothing when no file was lent.
    Lent(Option<Seen>),
    /// How many kinds a language counts, and how many kinds, fields and fixed names it states; then the names at kind 2, field
    /// 1 and fixed place 0.
    Names([usize; 4], [&'static str; 3]),
    /// The name at the place of the field code the table holds for the file's first word.
    Field(&'static str),
    /// The id after the file's own tree, the table's elements and how many embedded segments it has; then the first segment's
    /// host, root and end (zeros when it has none).
    Embedded([usize; 3], [u32; 3]),
}

/// The visit of the cases: fills `syntax-error` once and keeps what it saw.
#[derive(Default)]
struct Seeing(Option<Seen>);

impl FileVisit for Seeing {
    fn visit(&mut self, file: FileFacts<'_>) {
        let mut words =
            file_words(&base_words(&FixedNames::default(), &[]).expect("no fixed name to refuse"));
        let mut columns = FilledColumns::default();
        let answer = file.facts.fill(
            Relation::SyntaxError,
            Wanted::Whole,
            &mut words,
            &mut columns,
        );
        self.0 = Some(Seen {
            file: file.file,
            nodes: node_count(file.table),
            elements: element_count(file.table),
            syntax_errors: Count(columns.0.first().map_or(0, |column| column.0.len())),
            answer,
        });
    }
}

/// The visit of the table cases: keeps the name of the field the first word fills, or where the segments are.
struct Reading {
    /// Which of the two a row reads.
    scenario: Scenario,
    /// What it read.
    answer: Option<Answer>,
}

impl FileVisit for Reading {
    fn visit(&mut self, file: FileFacts<'_>) {
        let first = Position(u32::try_from(node_count(file.table).0).unwrap_or(u32::MAX));
        let code = usize::from(field_of(file.table, first).0);
        self.answer = Some(match self.scenario {
            Scenario::Embedded(_) => Answer::Embedded(
                [
                    usize::from(own_end(file.table)),
                    element_count(file.table).0,
                    embedded_segments(file.table).count(),
                ],
                embedded_segments(file.table)
                    .next()
                    .map_or([0; 3], |each| [each.host.0, each.root.0, each.end.0]),
            ),
            _ => Answer::Field(
                Walking
                    .vocabulary()
                    .fields
                    .0
                    .get(code)
                    .map_or("no such place", |name| name.0),
            ),
        });
    }
}

type Input = Scenario;
type Value = Answer;
type Failure = WalkError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_of_words_is_walked_and_lent_with_its_fillers"),
        word: Word::Succeeds,
        input: || Scenario::Words("one two! three"),
        expect: Expect::Value(Answer::Lent(Some(Seen {
            file: FileId(1),
            nodes: Count(1),
            elements: Count(4),
            syntax_errors: Count(1),
            answer: FactAnswer::Answered(FillExtent::Asked),
        }))),
    },
    Case {
        name: CaseName("a_held_language_walks_file_after_file_into_one_workers_buffers"),
        word: Word::Succeeds,
        input: || Scenario::Held("one two! three"),
        expect: Expect::Value(Answer::Lent(Some(Seen {
            file: FileId(1),
            nodes: Count(1),
            elements: Count(4),
            syntax_errors: Count(1),
            answer: FactAnswer::Answered(FillExtent::Asked),
        }))),
    },
    Case {
        name: CaseName("a_held_language_names_its_kinds_fields_and_fixed_words_by_place"),
        word: Word::Succeeds,
        input: || Scenario::Names,
        expect: Expect::Value(Answer::Names([4, 4, 2, 1], ["word", "label", "next-token"])),
    },
    Case {
        name: CaseName("the_language_that_reads_nothing_names_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NamesOfNothing,
        expect: Expect::Value(Answer::Names([0, 0, 0, 0], ["", "", ""])),
    },
    Case {
        name: CaseName("a_fields_code_is_the_place_of_its_name"),
        word: Word::Succeeds,
        input: || Scenario::Field("name: one"),
        expect: Expect::Value(Answer::Field("label")),
    },
    Case {
        name: CaseName("a_word_that_fills_no_field_has_the_code_of_no_field"),
        word: Word::Succeeds,
        input: || Scenario::Field("one two"),
        expect: Expect::Value(Answer::Field("")),
    },
    Case {
        name: CaseName("a_quote_is_an_embedded_segment_after_the_files_own_tree"),
        word: Word::Succeeds,
        input: || Scenario::Embedded("one <two> three"),
        expect: Expect::Value(Answer::Embedded([5, 7, 1], [1, 5, 7])),
    },
    Case {
        name: CaseName("a_file_with_no_quote_ends_its_own_tree_at_its_last_element"),
        word: Word::Succeeds,
        input: || Scenario::Embedded("one two"),
        expect: Expect::Value(Answer::Embedded([3, 3, 0], [0, 0, 0])),
    },
    Case {
        name: CaseName("the_language_that_reads_nothing_refuses_held_too"),
        word: Word::Refuses,
        input: || Scenario::HeldNothing,
        expect: Expect::Error(WalkError::Unsupported),
    },
    Case {
        name: CaseName("the_language_that_reads_nothing_refuses_a_file"),
        word: Word::Refuses,
        input: || Scenario::Nothing,
        expect: Expect::Error(WalkError::Unsupported),
    },
    Case {
        name: CaseName("a_parser_that_fails_is_the_languages_failure"),
        word: Word::Fails,
        input: || Scenario::Failing,
        expect: Expect::Error(WalkError::Library),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (bytes, path): (&[u8], &str) = match scenario {
        Scenario::Words(text)
        | Scenario::Held(text)
        | Scenario::Field(text)
        | Scenario::Embedded(text) => (text.as_bytes(), "words.walk"),
        Scenario::Failing => (FAILING, "broken.walk"),
        Scenario::Nothing | Scenario::HeldNothing | Scenario::Names | Scenario::NamesOfNothing => {
            (b"", "src/lib.rs")
        }
    };
    let request = WalkRequest {
        path: SourcePath(path),
        file: FileId(1),
        source: ByteSlice(bytes),
    };
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(4));
    let mut seeing = Seeing::default();
    match scenario {
        Scenario::Names | Scenario::NamesOfNothing => {
            let language: &dyn WalkEngine = match scenario {
                Scenario::Names => &Walking,
                _ => &WalkUseCaseImpl,
            };
            let names = language.vocabulary();
            let at = |list: &[ryo_relations::domain::terms::types::NameText<'static>],
                      place: usize| {
                list.get(place).map_or("", |name| name.0)
            };
            return Ok(Answer::Names(
                [
                    language.kind_count().0,
                    names.kinds.0.len(),
                    names.fields.0.len(),
                    names.fixed.0.len(),
                ],
                [
                    at(&names.kinds.0, 2),
                    at(&names.fields.0, 1),
                    at(&names.fixed.0, 0),
                ],
            ));
        }
        Scenario::Field(_) | Scenario::Embedded(_) => {
            let mut reading = Reading {
                scenario,
                answer: None,
            };
            Walking.walk(
                request,
                RelationNeeds(&[]),
                &mut writer,
                &mut (),
                &mut reading,
            )?;
            return Ok(reading.answer.unwrap_or(Answer::Lent(None)));
        }
        Scenario::Held(_) | Scenario::HeldNothing => {
            let language: &dyn WalkEngine = match scenario {
                Scenario::Held(_) => &Walking,
                _ => &WalkUseCaseImpl,
            };
            let (held, mut buffers, needs) = (
                &mut NodeTable::default(),
                language.buffers(),
                RelationNeeds(&[]),
            );
            let first = WalkRequest {
                source: ByteSlice(b"one"),
                ..request
            };
            language.walk_file(first, needs, held, &mut *buffers, &mut Seeing::default())?;
            language.walk_file(request, needs, held, &mut *buffers, &mut seeing)?;
        }
        Scenario::Nothing => WalkUseCaseImpl.walk(
            request,
            RelationNeeds(&[]),
            &mut writer,
            &mut (),
            &mut seeing,
        )?,
        Scenario::Words(_) | Scenario::Failing => {
            Walking.walk(
                request,
                RelationNeeds(&[]),
                &mut writer,
                &mut (),
                &mut seeing,
            )?;
        }
    }
    Ok(Answer::Lent(seeing.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
