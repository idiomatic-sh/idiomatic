//! Every publication adapter preserves complete rows and refuses reserved arenas.
use super::super::interface::PublicationPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CROSS_FILE, CellKind, Integer, Value as Cell, WorkerNumber,
};
use ryo_values::domain::{
    records::types::ObservationReason,
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};

#[derive(Clone, Copy)]
pub enum Scenario {
    Numeric,
    Reserved,
}
type Input = Scenario;
type Value = ();
/// What a refusal says: a reason, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The adapter names why it refused.
    Reasoned,
    /// The adapter refused without a reason.
    Unreasoned,
}
type Failure = Refusal;
pub const CASES: &[Case<Input, Value, Failure>] = &[
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
        expect: Expect::Error(Refusal::Reasoned),
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
        answer.map_err(|reason: ObservationReason| {
            if reason.0.is_empty() {
                Refusal::Unreasoned
            } else {
                Refusal::Reasoned
            }
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
