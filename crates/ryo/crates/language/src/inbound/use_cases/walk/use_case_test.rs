//! What the `walk` use case promises: a language walks a file into the node table and lends it, with its fillers, to the visit;
//! the language that reads nothing refuses every file, and a parser that fails is the language's fault.

use super::errors::WalkError;
use super::fixtures::FAILING;
use super::interface::WalkUseCase as _;
use super::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use super::use_case::WalkUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{FactAnswer, FileFacts, FilledColumns};
use crate::domain::files::types::SourcePath;
use crate::domain::fillers::fixtures::Walking;
use crate::domain::node_table::service::{builder, element_count, node_count};
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

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
}

/// What the visit saw of the lent file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    /// The file's syntax nodes.
    nodes: Count,
    /// Its nodes and tokens.
    elements: Count,
    /// The `syntax-error` rows its filler wrote.
    syntax_errors: Count,
    /// Whether the filler answered every row.
    answer: FactAnswer,
}

/// The visit of the cases: fills `syntax-error` once and keeps what it saw.
#[derive(Default)]
struct Seeing(Option<Seen>);

impl FileVisit for Seeing {
    fn visit(&mut self, file: FileFacts<'_>) {
        let mut words = file_words(&base_words(&[]));
        let mut columns = FilledColumns::default();
        let answer = file
            .facts
            .fill(Relation::SyntaxError, &mut words, &mut columns);
        self.0 = Some(Seen {
            nodes: node_count(file.table),
            elements: element_count(file.table),
            syntax_errors: Count(columns.0.first().map_or(0, |column| column.0.len())),
            answer,
        });
    }
}

type Input = Scenario;
type Value = Option<Seen>;
type Failure = WalkError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_of_words_is_walked_and_lent_with_its_fillers"),
        word: Word::Succeeds,
        input: || Scenario::Words("one two! three"),
        expect: Expect::Value(Some(Seen {
            nodes: Count(1),
            elements: Count(4),
            syntax_errors: Count(1),
            answer: FactAnswer::Answered,
        })),
    },
    Case {
        name: CaseName("a_held_language_walks_file_after_file_into_one_workers_buffers"),
        word: Word::Succeeds,
        input: || Scenario::Held("one two! three"),
        expect: Expect::Value(Some(Seen {
            nodes: Count(1),
            elements: Count(4),
            syntax_errors: Count(1),
            answer: FactAnswer::Answered,
        })),
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
        Scenario::Words(text) | Scenario::Held(text) => (text.as_bytes(), "words.walk"),
        Scenario::Failing => (FAILING, "broken.walk"),
        Scenario::Nothing | Scenario::HeldNothing => (b"", "src/lib.rs"),
    };
    let request = WalkRequest {
        path: SourcePath(path),
        file: FileId(1),
        source: ByteSlice(bytes),
    };
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(3));
    let mut seeing = Seeing::default();
    match scenario {
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
    Ok(seeing.0)
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
