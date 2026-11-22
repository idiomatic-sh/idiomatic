//! The steps of the `walk` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_language::domain::facts::service::{clear_words, intern};
use ryo_language::domain::facts::types::{FileFacts, FilledColumns, Wanted};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::fillers::fixtures::growth::{GrowthReport, grown};
use ryo_language::domain::fillers::types::Fillers as _;
use ryo_language::domain::node_table::service::{builder, element_count, node_count};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::fixtures::{
    fill_named, fixture_words, held, link_kind,
};
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase as _;
use ryo_language::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkRequest};
use ryo_relations::domain::relations::types::{Count, FileId, Relation};
use ryo_relations::domain::terms::types::NameText;

use crate::Behavior;

/// A visit that hands the lent file to a closure.
struct Lent<'v>(&'v mut dyn FnMut(FileFacts<'_>));

impl FileVisit for Lent<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// Walks `text` as the file at `path` through the fixture language, lending it to `visit`; answers why the walk refused.
fn walked(path: &str, text: &str, visit: &mut dyn FnMut(FileFacts<'_>)) -> Option<String> {
    let request = WalkRequest {
        selection: ryo_language::inbound::use_cases::walk::types::WalkSelection::Recognized,
        path: SourcePath(path),
        file: FileId(1),
        source: ByteSlice(text.as_bytes()),
    };
    let (mut table, needs) = (NodeTable::default(), RelationNeeds(&[]));
    let writer = &mut builder(&mut table, Count(4));
    Walking
        .walk(request, needs, writer, &mut (), &mut Lent(visit))
        .err()
        .map(|error| error.to_string())
}

/// The scene's path and text: a path, a colon, and the file's text.
fn scene(input: &str) -> (&str, &str) {
    input.split_once(':').unwrap_or((input, ""))
}

/// The growth harness over files of the fixture language whose last word is the scene's text.
fn grew(input: &str, elements: usize) -> GrowthReport {
    let (path, last) = scene(input);
    grown(
        &Walking::vocabulary().fixed,
        Count(elements),
        &mut |asked, visit| {
            let text = format!("{}{last}", "w ".repeat(asked.0.saturating_sub(2)));
            let _refused = walked(path, &text, visit);
        },
    )
}

/// Names the scene the next `walk` step runs: a path, a colon, and the file's text.
#[given(expr = "the walk input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Walks the scene's file through the fixture language, keeping what the visit saw or why the walk refused.
#[when(expr = "walk runs")]
fn runs(world: &mut Behavior) {
    let (path, text) = scene(&world.input);
    let mut seen = String::new();
    let refused = walked(path, text, &mut |file| {
        let mut words = match fixture_words() {
            Ok(words) => words,
            Err(refused) => {
                seen = refused.to_string();
                return;
            }
        };
        let mut columns = FilledColumns::default();
        let _answer = file.facts.fill(
            Relation::SyntaxError,
            Wanted::Whole,
            &mut words,
            &mut columns,
        );
        let errors = columns.0.first().map_or(0, |column| column.0.len());
        seen = format!(
            "walked {} nodes, {} elements, {errors} syntax errors",
            node_count(file.table).0,
            element_count(file.table).0
        );
    });
    world.answer = refused.unwrap_or(seen);
}

/// Walks the scene's file and fills what the step names, keeping what the fill answered.
#[when(expr = "walk fills {string}")]
fn fills(world: &mut Behavior, asked: String) {
    let (path, text) = scene(&world.input);
    let mut seen = None;
    let refused = walked(path, text, &mut |mut file| {
        seen = fill_named(&mut file, &asked)
    });
    world.answer = refused
        .or(seen)
        .unwrap_or_else(|| format!("nothing is named `{asked}`"));
}

/// Walks the scene's file and then a second with one worker's words, cleared between them and holding each file's own words
/// (as its other fills would intern them) before its links are read, keeping the kind of each one's links.
#[when(expr = "walk fills the links of the file and then of a file of {string}")]
fn fills_two(world: &mut Behavior, second: String) {
    let mut words = match fixture_words() {
        Ok(words) => words,
        Err(refused) => {
            world.answer = refused.to_string();
            return;
        }
    };
    let ((path, first), mut kinds) = (scene(&world.input), Vec::new());
    for each in [first, second.as_str()] {
        clear_words(&mut words);
        let _own: Vec<_> = each
            .split(' ')
            .map(|word| intern(&mut words, NameText(word)))
            .collect();
        let _refused = walked(path, each, &mut |mut file| {
            kinds.push(link_kind(&mut file, &mut words))
        });
    }
    world.answer = kinds.join(", then ");
}

/// Runs the growth harness over files of the fixture language that end as the scene's text, keeping the part asked for.
#[when(
    expr = "the growth harness holds the {string} of files of {int} elements and of four times as many"
)]
fn grows(world: &mut Behavior, part: String, elements: usize) {
    world.answer = held(&grew(&world.input, elements), &part);
}

/// Asserts what `walk` answered.
#[then(expr = "walk answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
