//! The steps of the `parse` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::service::build;
use ryo_language::inbound::use_cases::parse::types::ParseRequest;
use ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;
use ryo_relations::domain::relations::types::FileId;

use crate::Behavior;

/// Names the scene the next `parse` step runs.
#[given(expr = "the parse input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `parse` over the scene the input names, keeping what it answered.
#[when(expr = "parse runs")]
fn runs(world: &mut Behavior) {
    let request = ParseRequest {
        path: SourcePath(&world.input),
        file: FileId(0),
        bytes: ByteSlice(b"anything"),
        notes: ryo_language::inbound::use_cases::parse::types::NoteRequest::Unwanted,
        fields: ryo_language::inbound::use_cases::parse::types::FieldRequest::Unwanted,
    };
    world.answer = match build(
        &ParseUseCaseImpl,
        request,
        &mut Elements::default(),
        &mut Symbols::default(),
    ) {
        Ok(()) => "read".to_owned(),
        Err(error) => error.to_string(),
    };
}

/// Asserts the last `parse` step's answer.
#[then(expr = "parse answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber gives a captured string by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "parse answered otherwise");
}
