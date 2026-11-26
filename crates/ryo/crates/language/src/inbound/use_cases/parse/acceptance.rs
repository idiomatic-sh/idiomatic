//! The steps of the `parse` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest, Recognition};
use ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;
use ryo_relations::domain::relations::types::FileId;

use crate::Behavior;

/// The scene that asks how many kinds the language has.
const KINDS_SCENE: &str = "contract.kinds";

/// What the language answers for a file it does not read.
const DECLINED: &str = "the language does not read this file";

/// Names the scene the next `parse` step runs.
#[given(expr = "the parse input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `parse` over the scene the input names, keeping what it answered.
#[when(expr = "parse runs")]
fn runs(world: &mut Behavior) {
    let language: &dyn ParseEngine = &ParseUseCaseImpl;
    world.answer = if world.input == KINDS_SCENE {
        format!("{} kinds", language.kind_count().0)
    } else {
        let request = ParseRequest {
            path: SourcePath(&world.input),
            file: FileId(0),
            bytes: ByteSlice(b""),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        match language.recognition(&request) {
            Recognition::Declines => DECLINED.to_owned(),
            Recognition::Reads => "the language reads this file".to_owned(),
        }
    };
}

/// Asserts the answer.
#[then(expr = "parse answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
