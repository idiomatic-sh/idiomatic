//! The steps of the `publication` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::publication::interface::{PublicationInput, PublicationUseCase};
use ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CROSS_FILE, CellKind, Integer, Value, WorkerNumber};
use ryo_values::domain::{rows::types::{RowCount, Table}, stored_facts::types::ArenaBytes};

#[given(expr = "a publication from worker {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "publication runs")]
fn runs(world: &mut Behavior) {
    let mut table = Table { kinds: vec![CellKind::Number], cells: vec![Value::Number(Integer(17))], count: RowCount(1) };
    let worker = if world.input == "cross-file" { CROSS_FILE } else { WorkerNumber(0) };
    let (mut symbols, mut arena, mut places) = (Symbols::default(), ArenaBytes::default(), Vec::new());
    let answer = PublicationUseCaseImpl::new().publication(PublicationInput { table: &mut table, symbols: &mut symbols, inherited: &[], worker, arena: &mut arena, places: &mut places });
    world.answer = if answer.is_ok() { "published" } else { "refused" }.to_owned();
}

#[then(expr = "publication answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
