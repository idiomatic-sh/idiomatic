//! The `blocks` service's cases, on values alone: the blocks a document makes, read from the rows any language of
//! documents gives of the shared relations, and how a document departs from its exact template.

use super::service::{blocks, sequence};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::comments::types::ByteSpan;
use crate::domain::contracts::service::facts;
use crate::domain::contracts::types::{
    ComparedText, DataRow, RowKey, RowNode, RowPlace, Syntax, SyntaxKind, TemplateIdentity,
    WrittenValue,
};
use crate::domain::findings::types::ByteOffset;

/// A document of a section, a run of prose on one line and a literal opened by a line of its own.
const DOCUMENT: &str = "# demo\n\nOne line.\n\n~~~ one\nx\n~~~\n";

/// The same document, its literal opened by another line.
const OTHER: &str = "# demo\n\nOne line.\n\n~~~ two\nx\n~~~\n";

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The blocks of a document, by kind.
    Blocks,
    /// How many messages a document whose literal is opened by another line departs with.
    Opened,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_document_is_its_sections_runs_and_literals_in_order"),
        word: Word::Succeeds,
        input: || Question::Blocks,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_literal_opened_by_another_line_is_missing_and_extra"),
        word: Word::Succeeds,
        input: || Question::Opened,
        expect: Expect::Value(String::new()),
    },
];

/// A row of `syntax`, at `start..end`.
fn row(
    syntax: Syntax,
    id: &str,
    (key, kind, text): (&str, &str, &str),
    (start, end): (usize, usize),
) -> DataRow {
    DataRow {
        syntax,
        id: RowNode(id.into()),
        owner: RowNode("0".into()),
        key: RowKey(key.into()),
        kind: SyntaxKind(kind.into()),
        text: WrittenValue(text.into()),
        span: RowPlace::At(ByteSpan {
            start: ByteOffset(start),
            end: ByteOffset(end),
        }),
    }
}

/// The rows of `DOCUMENT` and of `OTHER`, as a language of documents gives them.
fn rows() -> Vec<DataRow> {
    vec![
        row(Syntax::Scopes, "0", ("", "", ""), (0, 33)),
        row(Syntax::Defines, "1", ("demo", "section", "heading"), (0, 7)),
        row(Syntax::Scopes, "1", ("", "", ""), (0, 33)),
        row(Syntax::Prose, "1", ("", "", "demo"), (0, 7)),
        row(Syntax::Prose, "2", ("", "", "One line."), (8, 18)),
        row(Syntax::Literals, "3", ("", "", "x\n"), (19, 33)),
    ]
}

fn subject(question: Input) -> Value {
    let rows = rows();
    let (said, expected) = match question {
        Question::Blocks => (
            blocks(&facts(&rows), ComparedText(DOCUMENT))
                .iter()
                .map(|block| format!("{}:{}", block.kind.0, block.attributes.len()))
                .collect::<Vec<_>>()
                .join(" "),
            "section-1:0 prose:1 literal:1".to_owned(),
        ),
        Question::Opened => (
            sequence(
                &blocks(&facts(&rows), ComparedText(DOCUMENT)),
                &blocks(&facts(&rows), ComparedText(OTHER)),
                &TemplateIdentity("p/t".into()),
            )
            .len()
            .to_string(),
            "2".to_owned(),
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
