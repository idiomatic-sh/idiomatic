//! The real adapter satisfies the port and carries producer symbols into its arena.
use super::adapter::PublicationExecutionAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::publication::{fixtures::contract, interface::PublicationPort};
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, Value as Cell, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};

type Input = ();
type Value = usize;
type Failure = ryo_values::domain::records::types::ObservationReason;
const NAME_BYTES: &[u8] = &[b'a', b'l', b'p', b'h', b'a'];
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(symbol_gets_portable_text_and_place)),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(1),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    let adapter = PublicationExecutionAdapter::new();
    let mut symbols = Symbols::default();
    let name = std::str::from_utf8(NAME_BYTES).unwrap();
    let symbol = intern_derived(&mut symbols, ByteSlice(&[]), NameText(name));
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        cells: vec![Cell::Symbol(symbol)],
        count: RowCount(1),
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    adapter.publication(
        &mut table,
        &mut symbols,
        &[],
        WorkerNumber(0),
        &mut arena,
        &mut places,
    )?;
    assert!(matches!(table.cells[0], Cell::Name(_)));
    assert_eq!(arena.0, [vec![7], NAME_BYTES.to_vec()].concat());
    Ok(places.len())
}
#[test]
fn succeeds() {
    contract::check(&PublicationExecutionAdapter::new());
    run(CASES, Word::Succeeds, subject);
}
