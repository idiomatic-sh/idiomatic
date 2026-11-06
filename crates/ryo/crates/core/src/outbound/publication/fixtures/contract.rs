//! Every publication adapter preserves complete rows and refuses reserved arenas.
use super::super::interface::PublicationPort;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CROSS_FILE, CellKind, Integer, Value as Cell, WorkerNumber,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
    records::types::ObservationReason,
};

#[derive(Clone, Copy)]
enum Scenario {
    Numeric,
    Reserved,
}
type Input = Scenario;
type Value = ();
type Failure = ObservationReason;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(numeric_row_remains_complete)),
        word: Word::Succeeds,
        input: || Scenario::Numeric,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(reserved_arena_refuses_without_mutation)),
        word: Word::Refuses,
        input: || Scenario::Reserved,
        expect: Expect::Refusal(|reason| selects(!reason.0.is_empty())),
    },
];

pub fn check(adapter: &impl PublicationPort) {
    let subject = |scenario| {
        let mut table = Table {
            kinds: vec![CellKind::Number],
            cells: vec![Cell::Number(Integer(17))],
            count: RowCount(1),
        };
        let before = table.cells.clone();
        let mut symbols = Symbols::default();
        let mut arena = ArenaBytes(vec![7]);
        let mut places = Vec::new();
        let worker = if matches!(scenario, Scenario::Reserved) {
            CROSS_FILE
        } else {
            WorkerNumber(0)
        };
        let answer = adapter.publication(
            &mut table,
            &mut symbols,
            &[],
            worker,
            &mut arena,
            &mut places,
        );
        assert_eq!(table.cells, before);
        assert_eq!(arena.0, [7]);
        assert!(places.is_empty());
        answer
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
