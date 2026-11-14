//! The steps of the `parse` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::flow_table::service::flow_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::fixtures::flowing::{Flowing, KEPT_GRAPH};
use ryo_language::inbound::use_cases::parse::service::build;
use ryo_language::inbound::use_cases::parse::types::{ParseEngine, ParseRequest};
use ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;
use ryo_relations::domain::relations::types::FileId;

use crate::Behavior;

/// The start of the name of a scene read by the language whose read keeps a control flow graph.
const FLOW_SCENE: &str = "contract.flow-";

/// What a build that read the file answers.
const READ: &str = "read";

/// Names the scene the next `parse` step runs.
#[given(expr = "the parse input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `parse` over the scene the input names, keeping what it answered. A flow scene (`contract.flow-*`) is read by the language
/// whose read keeps a graph, on a table that first read a file with one, and its answer adds what the table keeps of the graph.
#[when(expr = "parse runs")]
fn runs(world: &mut Behavior) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let flowing = world.input.starts_with(FLOW_SCENE);
    let language: &dyn ParseEngine = if flowing { &Flowing } else { &ParseUseCaseImpl };
    if flowing {
        let kept = format!("{FLOW_SCENE}{KEPT_GRAPH}");
        if let Err(error) = build(language, request(&kept), &mut elements, &mut symbols) {
            world.answer = error.to_string();
            return;
        }
    }
    world.answer = outcome(build(
        language,
        request(&world.input),
        &mut elements,
        &mut symbols,
    ));
    if flowing && world.answer == READ {
        let graph = flow_of(&elements);
        world.answer = match graph.edges.len() {
            0 => format!("{READ}, {} flow blocks", graph.blocks.len()),
            1 => format!("{READ}, {} flow blocks, 1 edge", graph.blocks.len()),
            edges => format!("{READ}, {} flow blocks, {edges} edges", graph.blocks.len()),
        };
    }
}

/// The request to read the file at `path`.
fn request(path: &str) -> ParseRequest<'_> {
    ParseRequest {
        path: SourcePath(path),
        file: FileId(0),
        bytes: ByteSlice(b"anything"),
        notes: ryo_language::inbound::use_cases::parse::types::NoteRequest::Unwanted,
        fields: ryo_language::inbound::use_cases::parse::types::FieldRequest::Unwanted,
    }
}

/// What a build answered, as the feature states it.
fn outcome(built: Result<(), ParseError>) -> String {
    match built {
        Ok(()) => READ.to_owned(),
        Err(error) => error.to_string(),
    }
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
