//! The `semantics` service's cases, on values alone.

use super::errors::SemanticsError;
use super::service::admit;
use super::types::{RenderContext, RenderRequest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Count, FileId, Position};

/// A request for the node at a position of a file, asked of the file number three that holds two elements.
type Input = (FileId, Position);
type Value = ();
type Failure = SemanticsError;

/// The request for the node at `position` of `file`, rendered as a positional argument.
fn request((file, position): Input) -> RenderRequest {
    RenderRequest {
        node: node_id(file, position),
        context: RenderContext::PositionalArgument,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_of_the_current_file_is_admitted"),
        word: Word::Succeeds,
        input: || (FileId(3), Position(1)),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_node_of_another_file_is_refused"),
        word: Word::Refuses,
        input: || (FileId(4), Position(0)),
        expect: Expect::Refusal(|error| selects!(error, SemanticsError::ForeignFile { .. })),
    },
    Case {
        name: CaseName("a_position_past_the_table_is_refused"),
        word: Word::Refuses,
        input: || (FileId(3), Position(2)),
        expect: Expect::Refusal(|error| selects!(error, SemanticsError::StaleNode { .. })),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    admit(&request(input), FileId(3), Count(2))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
