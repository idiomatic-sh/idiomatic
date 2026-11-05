//! The real adapter satisfies the port and carries producer symbols into its arena.
use super::adapter::PublicationExecutionAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::publication::{fixtures::contract, interface::PublicationPort};
use ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, Value, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};
use std::sync::Arc;

type Input = ();
type Answer = usize;
type Failure = ryo_values::domain::observations::types::ObservationReason;
const NAME_BYTES: &[u8] = &[b'a', b'l', b'p', b'h', b'a'];
const CASES: &[Case<Input, Answer, Failure>] = &[Case {
    name: CaseName(stringify!(symbol_gets_portable_text_and_place)),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(1),
}];
fn subject(_: Input) -> Result<Answer, Failure> {
    let adapter = PublicationExecutionAdapter::new(Arc::new(PublicationUseCaseImpl::new()));
    let mut symbols = Symbols::default();
    let name = std::str::from_utf8(NAME_BYTES).unwrap();
    let symbol = intern_derived(&mut symbols, ByteSlice(&[]), NameText(name));
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        cells: vec![Value::Symbol(symbol)],
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
    assert!(matches!(table.cells[0], Value::Name(_)));
    assert_eq!(arena.0, [vec![7], NAME_BYTES.to_vec()].concat());
    Ok(places.len())
}
#[test]
fn succeeds() {
    contract::check(&PublicationExecutionAdapter::new(Arc::new(
        PublicationUseCaseImpl::new(),
    )));
    run(CASES, Word::Succeeds, subject);
}
