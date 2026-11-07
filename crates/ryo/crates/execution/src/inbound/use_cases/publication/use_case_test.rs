//! The engine's publication carries producer symbols into the caller's arena and refuses the reserved arena untouched.
use super::{interface::PublicationUseCase, use_case::PublicationUseCaseImpl};
use crate::application::publication::{errors::PublicationError, types::PublicationInput};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish::types::PublicationReason;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CROSS_FILE, CellKind, Integer, Value as Cell, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};

/// What a producer publishes, and from which worker.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A row of one producer symbol, from a producing worker.
    Symbol,
    /// A row of one number, from a producing worker.
    Numeric,
    /// A row of one number, from the reserved cross-file worker.
    Reserved,
}
type Input = Scenario;
/// The places the publication wrote.
type Value = usize;
type Failure = PublicationError;
const NAME_BYTES: &[u8] = &[b'a', b'l', b'p', b'h', b'a'];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(symbol_gets_portable_text_and_place)),
        word: Word::Succeeds,
        input: || Scenario::Symbol,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(stringify!(numeric_row_remains_complete)),
        word: Word::Succeeds,
        input: || Scenario::Numeric,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(stringify!(reserved_arena_refuses_without_mutation)),
        word: Word::Refuses,
        input: || Scenario::Reserved,
        expect: Expect::Error(PublicationError::Publication {
            reason: PublicationReason::ReservedArena,
        }),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let mut table = match scenario {
        Scenario::Symbol => {
            let name = std::str::from_utf8(NAME_BYTES).unwrap();
            let symbol = intern_derived(&mut symbols, ByteSlice(&[]), NameText(name));
            Table {
                kinds: vec![CellKind::Symbol],
                cells: vec![Cell::Symbol(symbol)],
                count: RowCount(1),
            }
        }
        Scenario::Numeric | Scenario::Reserved => Table {
            kinds: vec![CellKind::Number],
            cells: vec![Cell::Number(Integer(17))],
            count: RowCount(1),
        },
    };
    let before = table.cells.clone();
    let worker = if matches!(scenario, Scenario::Reserved) {
        CROSS_FILE
    } else {
        WorkerNumber(0)
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    let answer = PublicationUseCaseImpl::new().publication(PublicationInput {
        table: &mut table,
        symbols: &mut symbols,
        inherited: &[],
        worker,
        arena: &mut arena,
        places: &mut places,
    });
    if matches!(scenario, Scenario::Symbol) {
        assert!(matches!(table.cells[0], Cell::Name(_)));
        assert_eq!(arena.0, [vec![7], NAME_BYTES.to_vec()].concat());
    } else {
        assert_eq!(table.cells, before);
        assert_eq!(arena.0, [7]);
        assert!(places.is_empty());
    }
    if let Err(error) = &answer {
        assert!(!error.to_string().is_empty(), "a refusal names its reason");
    }
    answer.map(|()| places.len())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
