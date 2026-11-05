//! The reading of an item's rows, on values alone: every row as typed cells, and each cause once, a cause of one file of a run with
//! the file's path.

use super::super::contract::Collect;
use super::super::expected::{want_file, want_flag, want_name, want_node, want_span, want_unknown};
use super::{Reading, written};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::SourcePath;
use ryo_relations::domain::relations::collections::relations::comment::item::CommentRow;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    FileId, Flag, Missing, Offset, Position, Relation, ReportText, Rows, Span,
};
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::terms::service::unknown_symbol;
use ryo_relations::domain::terms::types::{NameText, ReasonName, UnknownName, UnknownReason};

/// What a row gives: the files of the run, by number, whether the causes are said of one of them, and what the row's text is.
type Input = (&'static [&'static str], Said, Text);
/// What the item wrote, read back.
type Value = Reading;
type Failure = ();

/// Where the causes of a case are said.
#[derive(Clone, Copy, Debug)]
enum Said {
    /// Of the relation, twice.
    OfTheRelation,
    /// Of the file numbered 0, twice.
    OfTheFile,
}

/// What the text of the row is.
#[derive(Clone, Copy, Debug)]
enum Text {
    /// A name.
    Name,
    /// A value the language cannot resolve: the first reason it declares.
    Unknown,
}

/// The reasons the language of the cases declares for a value it cannot resolve.
const REASONS: &[UnknownName] = &[UnknownName {
    reason: UnknownReason(0),
    text: ReasonName("not-a-note"),
}];

/// The path the one file of a case is read as.
const SHOWN: &str = "item.txt";
/// The text of the one file of a case.
const TEXT: &[u8] = b"# note\n";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_is_its_cells_and_a_cause_said_twice_is_shown_once"),
        word: Word::Succeeds,
        input: || (&[], Said::OfTheRelation, Text::Name),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("note"),
                want_file(SHOWN),
                want_span((1, 1), (1, 7)),
                want_flag(false),
            ]],
            causes: &["the comment is read in part"],
        }),
    },
    Case {
        name: CaseName("a_cause_said_of_one_file_of_a_run_is_shown_with_its_path"),
        word: Word::Succeeds,
        input: || (&["docs/item.txt"], Said::OfTheFile, Text::Name),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name("note"),
                want_file("docs/item.txt"),
                want_span((1, 1), (1, 7)),
                want_flag(false),
            ]],
            causes: &["the comment is read in part (in docs/item.txt)"],
        }),
    },
    Case {
        name: CaseName("an_unknown_cell_shows_the_name_of_the_reason_the_language_declares"),
        word: Word::Succeeds,
        input: || (&[], Said::OfTheRelation, Text::Unknown),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_unknown("not-a-note"),
                want_file(SHOWN),
                want_span((1, 1), (1, 7)),
                want_flag(false),
            ]],
            causes: &["the comment is read in part"],
        }),
    },
];

fn subject((files, said, text): Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let mut out = Collect::default();
    let text = match text {
        Text::Name => intern_derived(&mut symbols, ByteSlice(&[]), NameText("note")),
        Text::Unknown => unknown_symbol(UnknownReason(0)),
    };
    Rows::<CommentRow>::write(
        &mut out,
        CommentRow {
            id: node_id(FileId(0), Position(1)),
            text,
            file: FileId(0),
            span: Span {
                start: Offset(0),
                end: Offset(6),
            },
            doc: Flag::Lowered,
        },
    );
    for _ in 0_i32..2_i32 {
        let cause = ReportText("the comment is read in part");
        match said {
            Said::OfTheRelation => Rows::<CommentRow>::partial(&mut out, cause, Missing::Any),
            Said::OfTheFile => {
                Rows::<CommentRow>::partial_at(&mut out, FileId(0), cause, Missing::Any);
            }
        }
    }
    Ok(written(
        out,
        symbols,
        (files, TEXT),
        (SourcePath(SHOWN), Relation::Comment),
        REASONS,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
