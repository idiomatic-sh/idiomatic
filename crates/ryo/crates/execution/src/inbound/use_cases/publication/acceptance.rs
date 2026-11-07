//! The publication feature exercises the actual public use case and storage effects.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::publication::{
    interface::PublicationUseCase, types::PublicationInput, use_case::PublicationUseCaseImpl,
};
use ryo_language::domain::symbols::{
    fixtures::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CROSS_FILE, CellKind, Integer, Value, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};

#[given(expr = "a producer row {string}")]
fn producer(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "publication runs")]
fn runs(world: &mut Behavior) {
    let mut symbols = Symbols::default();
    let (cell, kind, worker) = match world.input.as_str() {
        "symbol" => (
            Value::Symbol(intern_derived(
                &mut symbols,
                ByteSlice(&[]),
                NameText("alpha"),
            )),
            CellKind::Symbol,
            WorkerNumber(0),
        ),
        "reserved" => (Value::Number(Integer(17)), CellKind::Number, CROSS_FILE),
        "numeric" => (
            Value::Number(Integer(17)),
            CellKind::Number,
            WorkerNumber(0),
        ),
        _ => panic!("unknown publication input"),
    };
    let mut table = Table {
        kinds: vec![kind],
        cells: vec![cell],
        count: RowCount(1),
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    let result = PublicationUseCaseImpl::new().publication(PublicationInput {
        table: &mut table,
        symbols: &mut symbols,
        inherited: &[],
        worker,
        arena: &mut arena,
        places: &mut places,
    });
    world.answer = if worker == CROSS_FILE {
        assert!(result.is_err());
        assert_eq!(table.cells, [cell]);
        assert_eq!(arena.0, [7]);
        assert!(places.is_empty());
        "refused unchanged"
    } else if world.input == "symbol" {
        result.expect("symbol publication succeeds");
        assert!(matches!(table.cells[0], Value::Name(_)));
        assert_eq!(arena.0, b"\x07alpha");
        assert_eq!(places.len(), 1);
        "portable symbol"
    } else {
        result.expect("numeric publication succeeds");
        assert_eq!(table.cells, [cell]);
        assert_eq!(arena.0, [7]);
        "portable numeric"
    }
    .to_owned();
}

#[then(expr = "publication answers {string}")]
fn answer(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
