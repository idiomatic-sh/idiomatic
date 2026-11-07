//! The `table_cells` service's cases, on values alone.

use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::{
    FileId, KindBits, Offset, Position, Span, Value as Cell,
};

use super::service::read_tree_cells;
use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, TreeForm, TreeRows};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::errors::RowsError;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

/// A row of a tree is decided on the cells asked for, and the others are left as they were.
fn reads_only_the_cells_asked_of_a_tree() -> Value {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(1), Offset(3));
    leave_node(&mut builder, Offset(4));
    let tree = TreeRows {
        elements: &elements,
        file: FileId(3),
        form: TreeForm::Elements,
    };
    let mut scratch = vec![Cell::Flag(Flag::Lowered); 5];
    let read = read_tree_cells(&tree, RowIndex(1), &mut scratch, &ColumnPlaces(vec![4, 0]));
    let beyond = read_tree_cells(&tree, RowIndex(2), &mut scratch, &ColumnPlaces(vec![0]));
    let id = Cell::Node(node_id(FileId(3), Position(1)));
    usize::from(
        read == CellsRead::Read
            && beyond == CellsRead::Missing
            && scratch[0] == id
            && scratch[4]
                == Cell::Span(Span {
                    start: Offset(1),
                    end: Offset(3),
                })
            && scratch[1] == Cell::Flag(Flag::Lowered),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_the_cells_asked_of_a_tree_are_read"),
    word: Word::Succeeds,
    input: || || Ok(reads_only_the_cells_asked_of_a_tree()),
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::RowIndex;

// The prepared derived payloads the crate's tests build their subjects from, through the real port.
use crate::outbound::derived::interface::DerivedPort as _;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use crate::domain::table_model::types::Rows;
/// Finite typed input adapter for pure semantic conformance, with no cache or scheduler.
pub(crate) struct SemanticRows<'f>(pub crate::domain::table_model::types::Rows<'f>);
impl ryo_derived::domain::row_cells::types::RowCells for SemanticRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::row_cells::types::BulkRows<'_>> {
        match &self.0 {
            Rows::Stored(source) => Some(ryo_derived::domain::row_cells::types::BulkRows::Stored(
                *source,
            )),
            Rows::Computed(table) => Some(
                ryo_derived::domain::row_cells::types::BulkRows::Computed(table),
            ),
            _ => None,
        }
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::fault::types::DerivedFault> {
        Ok(())
    }
    fn kinds(&self) -> &[CellKind] {
        crate::domain::table_model::service::kinds_of(&self.0)
    }
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        crate::domain::table_model::service::count_of(&self.0)
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Cell> {
        crate::domain::table_model::service::cell_at(&self.0, row, column)
    }
}
pub(crate) fn prepared_payload(
    request: ryo_derived::domain::request::types::DerivedRequest,
    rows: &Rows<'_>,
) -> Result<ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload, RowsError>
{
    use ryo_derived::domain::contract::types::BuildInputs;
    use ryo_derived::domain::contract::types::BuildOutcome;
    use ryo_derived::domain::contract::types::BuildWorkspace;
    use ryo_derived::domain::fault::types::DerivedFault;
    let port = std::sync::Arc::new(crate::outbound::derived::factory::semantics());
    let prepared = port.prepare(
        request,
        vec![crate::domain::table_model::service::kinds_of(rows).to_vec()],
    );
    let input = SemanticRows(rows.clone());
    match port.build(
        &prepared,
        BuildInputs::Rows(&input),
        &mut BuildWorkspace::default(),
        None,
    ) {
        Ok(BuildOutcome::Built(payload)) => Ok(payload),
        Err(DerivedFault::NoSuchColumn { column }) => {
            Err(RowsError::NoSuchColumn { index: column.0 })
        }
        other => panic!("unexpected semantic fixture result: {other:?}"),
    }
}
pub(crate) fn prepared_index(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<std::sync::Arc<KeyIndex>, RowsError> {
    prepared_payload(
        ryo_derived::domain::request::types::DerivedRequest::Equality {
            columns: columns.to_vec(),
        },
        rows,
    )
    .map(|payload| {
        payload
            .share_equality()
            .expect("successful equality payload")
            .0
    })
}

pub(crate) fn prepared_order(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<Vec<RowIndex>, RowsError> {
    use ryo_derived::domain::contract::types::DerivedView;
    use ryo_derived::domain::request::types::DerivedRequest;
    let request = DerivedRequest::PartitionOrder {
        keys: columns.to_vec(),
    };
    let payload = prepared_payload(request.clone(), rows)?;
    let port = std::sync::Arc::new(crate::outbound::derived::factory::semantics());
    let plan = port.prepare(
        request,
        vec![crate::domain::table_model::service::kinds_of(rows).to_vec()],
    );
    let DerivedView::PartitionOrder(order) = port.view(&plan, &payload).expect("prepared order")
    else {
        panic!("partition order")
    };
    Ok(order.to_vec())
}

// The small tree the crate's tests read elements from.
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_relations::domain::terms::types::NameText;
/// A tree of four elements: a `file` node holding the tokens `word`, `other` and `word`, in this order, at positions 1 to 3.
pub(crate) struct SmallTree {
    /// The elements.
    pub elements: Elements,
    /// The names.
    pub symbols: Symbols,
}

/// The bytes the small tree is of.
pub(crate) const SMALL_SOURCE: &[u8] = b"abcd";

/// Builds the small tree.
pub(crate) fn small_tree() -> SmallTree {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(SMALL_SOURCE));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(4));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
        name_kind(
            &mut completion(
                &mut elements,
                &mut symbols,
                ByteSlice(SMALL_SOURCE),
                FileId(0),
            ),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    SmallTree { elements, symbols }
}

/// The tree, as the rows the steps are run over.
pub(crate) fn rows_over(elements: &Elements) -> TreeRows<'_> {
    TreeRows {
        elements,
        file: FileId(0),
        form: TreeForm::Elements,
    }
}
