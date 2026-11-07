//! The publication service's real producer table and storage scenarios.
use super::{errors::PublicationError, service::publication, types::PublicationInput};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CROSS_FILE, CellKind, Integer, Value, WorkerNumber};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum Scenario {
    Number,
    Symbol,
    ReservedWorker,
}
const NAME_BYTES: &[u8] = &[b'a', b'l', b'p', b'h', b'a'];

pub(super) fn subject(
    scenario: Scenario,
) -> Result<(), PublicationError> {
    let mut symbols = Symbols::default();
    let cells = if matches!(scenario, Scenario::Symbol) {
        use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
        use ryo_relations::domain::terms::types::NameText;
        let name = std::str::from_utf8(NAME_BYTES).unwrap();
        vec![Value::Symbol(intern_derived(
            &mut symbols,
            ByteSlice(&[]),
            NameText(name),
        ))]
    } else {
        vec![Value::Number(Integer(17))]
    };
    let mut table = Table {
        kinds: vec![if matches!(scenario, Scenario::Symbol) {
            CellKind::Symbol
        } else {
            CellKind::Number
        }],
        cells: cells.clone(),
        count: RowCount(1),
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    let worker = if matches!(scenario, Scenario::ReservedWorker) {
        CROSS_FILE
    } else {
        WorkerNumber(0)
    };
    let result = publication(PublicationInput {
        table: &mut table,
        symbols: &mut symbols,
        inherited: &[],
        worker,
        arena: &mut arena,
        places: &mut places,
    });
    if matches!(scenario, Scenario::Symbol) {
        assert!(matches!(table.cells[0], Value::Name(_)));
        assert_eq!(arena.0, [vec![7], NAME_BYTES.to_vec()].concat());
        assert_eq!(places.len(), 1);
        assert_eq!(
            places[0].origin,
            ryo_relations::domain::relations::types::Origin::Derived(WorkerNumber(0))
        );
        assert_eq!((places[0].start.0, places[0].end.0), (1, 6));
    } else {
        assert_eq!(table.cells, cells);
        assert_eq!(arena.0, vec![7]);
        assert!(places.is_empty());
    }
    result
}
