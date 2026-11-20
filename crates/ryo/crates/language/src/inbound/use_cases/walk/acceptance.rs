//! The steps of the `walk` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::facts::service::{base_words, file_words};
use ryo_language::domain::facts::types::{FileFacts, FilledColumns};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::node_table::service::{builder, element_count, node_count};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase as _;
use ryo_language::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkRequest};
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

use crate::Behavior;

/// The visit of the feature: what the lent file holds and the `syntax-error` rows its filler writes.
#[derive(Default)]
struct Seeing(String);

impl FileVisit for Seeing {
    fn visit(&mut self, file: FileFacts<'_>) {
        let (mut words, mut columns) = (file_words(&base_words(&[])), FilledColumns::default());
        let _answer = file
            .facts
            .fill(Relation::SyntaxError, &mut words, &mut columns);
        let errors = columns.0.first().map_or(0, |column| column.0.len());
        self.0 = format!(
            "walked {} nodes, {} elements, {errors} syntax errors",
            node_count(file.table).0,
            element_count(file.table).0
        );
    }
}

/// Names the scene the next `walk` step runs: a path, a colon, and the file's text.
#[given(expr = "the walk input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Walks the scene's file through the fixture language, keeping what the visit saw or why the walk refused.
#[when(expr = "walk runs")]
fn runs(world: &mut Behavior) {
    let (path, text) = world
        .input
        .split_once(':')
        .unwrap_or((world.input.as_str(), ""));
    let request = WalkRequest {
        path: SourcePath(path),
        file: FileId(1),
        source: ByteSlice(text.as_bytes()),
    };
    let (mut table, mut seeing) = (NodeTable::default(), Seeing::default());
    world.answer = match Walking.walk(
        request,
        RelationNeeds(&[]),
        &mut builder(&mut table, Count(3)),
        &mut (),
        &mut seeing,
    ) {
        Ok(()) => seeing.0,
        Err(error) => error.to_string(),
    };
}

/// Asserts what `walk` answered.
#[then(expr = "walk answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
