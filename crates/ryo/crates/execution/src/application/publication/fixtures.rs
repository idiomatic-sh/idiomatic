//! The publication service's real producer table, published through one service entry, and what it left.
use super::{errors::PublicationError, types::PublicationInput};
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{
    CROSS_FILE, CellKind, Integer, NamePlace, Value, WorkerNumber,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};
use std::borrow::Cow;

/// The one cell a producer publishes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Produced {
    /// The number 17.
    Number,
    /// A producer symbol for the name `alpha`.
    Symbol,
}

/// The worker a publication runs on.
#[derive(Clone, Copy, Debug)]
pub(super) enum Worker {
    /// Producing worker 0.
    Producing,
    /// The reserved cross-file worker.
    Reserved,
}

/// What a producer publishes, and from which worker.
#[derive(Clone, Copy, Debug)]
pub(super) struct Publishing {
    pub(super) cell: Produced,
    pub(super) worker: Worker,
}

/// What the publication left: whether the cell became a portable name or stayed as it was, the
/// caller's arena (which held `[7]`) and the places written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Published {
    pub(super) named: bool,
    pub(super) unchanged: bool,
    pub(super) arena: Cow<'static, [u8]>,
    pub(super) places: Cow<'static, [NamePlace]>,
}

/// A refused publication: the error, its message, and what it left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Refused {
    pub(super) error: PublicationError,
    pub(super) message: Cow<'static, str>,
    pub(super) left: Published,
}

/// The name the symbol cell interns.
const NAME: &str = "alpha";

/// The service entry the publication runs through.
pub(super) type Publish = fn(PublicationInput<'_>) -> Result<(), PublicationError>;

/// Publishes the producer's one cell from its worker through `publication`, into an arena holding `[7]`.
pub(super) fn publish(publishing: Publishing, publication: Publish) -> Result<Published, Refused> {
    let mut symbols = Symbols::default();
    let (kind, cell) = match publishing.cell {
        Produced::Number => (CellKind::Number, Value::Number(Integer(17))),
        Produced::Symbol => (
            CellKind::Symbol,
            Value::Symbol(intern_derived(&mut symbols, ByteSlice(&[]), NameText(NAME))),
        ),
    };
    let mut table = Table {
        kinds: vec![kind],
        cells: vec![cell.clone()],
        count: RowCount(1),
    };
    let worker = match publishing.worker {
        Worker::Producing => WorkerNumber(0),
        Worker::Reserved => CROSS_FILE,
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    let answer = publication(PublicationInput {
        table: &mut table,
        symbols: &mut symbols,
        inherited: &[],
        worker,
        arena: &mut arena,
        places: &mut places,
    });
    let left = Published {
        named: matches!(table.cells[0], Value::Name(_)),
        unchanged: table.cells == [cell],
        arena: Cow::Owned(arena.0),
        places: Cow::Owned(places),
    };
    match answer {
        Ok(()) => Ok(left),
        Err(error) => Err(Refused {
            message: Cow::Owned(error.to_string()),
            error,
            left,
        }),
    }
}
