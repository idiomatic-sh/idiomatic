//! The `positions` service's cases, on values alone.
use ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase;

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::{FileId, Flag, KindBits, Offset, Position};
use ryo_relations::domain::terms::types::NameText;

use super::service::{count, in_tree, row, value};
use super::types::{Keeps, Keys, Positions, Slots};
use crate::domain::lowering::types::{ElementPart, RowWidth};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

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

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Orders three keys of two numbers.
    Sorted,
    /// Finds the rows of one key among them.
    Hits,
    /// Marks the first row of each key.
    Firsts,
    /// Counts the rows of positions three wide and reads the second.
    Rows,
    /// Reads the second row of positions that hold one.
    RowPastTheLast,
    /// Reads the parent of a position past the table.
    FieldPastTheTable,
    /// Places the root of the tree, and then a position past it.
    PlacePastTheTree,
}

/// What a row of the cases does.
type Input = Scenario;
/// The answer, shown.
type Value = &'static str;
type Failure = ();

/// The keys of three rows of two numbers: the second and the third are alike.
fn keys() -> Keys {
    Keys(vec![5, 1, 2, 9, 2, 9])
}

fn subject(scenario: Input) -> Value {
    let mut keep = Keeps::default();
    let shown = match scenario {
        Scenario::Sorted => {
            let payload = raw_order();
            let ryo_derived::domain::contract::types::DerivedView::RawKeyOrder(order) =
                payload.1.view(&payload.0, &payload.2).unwrap()
            else {
                panic!("raw order")
            };
            format!("{:?}", order.rows())
        }
        Scenario::Hits => {
            let payload = raw_order();
            let ryo_derived::domain::contract::types::DerivedView::RawKeyOrder(order) =
                payload.1.view(&payload.0, &payload.2).unwrap()
            else {
                panic!("raw order")
            };
            let found = order.hits(&[2, 9]);
            format!("{}..{}", found.start, found.end)
        }
        Scenario::Firsts => {
            let payload = raw_order();
            let ryo_derived::domain::contract::types::DerivedView::RawKeyOrder(order) =
                payload.1.view(&payload.0, &payload.2).unwrap()
            else {
                panic!("raw order")
            };
            order.firsts(&mut keep.0);
            format!("{:?}", keep.0)
        }
        Scenario::Rows => {
            let rows = Positions {
                width: RowWidth(3),
                slots: Slots(vec![1, 2, 3, 4, 5, 6]),
            };
            format!("{} {:?}", count(&rows).0, row(&rows, RowIndex(1)).0)
        }
        Scenario::RowPastTheLast => crash_of(|| {
            let rows = Positions {
                width: RowWidth(3),
                slots: Slots(vec![1, 2, 3]),
            };
            assert!(row(&rows, RowIndex(1)).0.is_empty());
        }),
        Scenario::PlacePastTheTree => {
            let tree = small_tree();
            let rows = rows_over(&tree.elements);
            let root = in_tree(&rows, Position(0));
            format!(
                "{} {}",
                root.0,
                crash_of(|| {
                    let past = in_tree(&rows, Position(4));
                    assert_eq!(past.0, 4);
                })
            )
        }
        Scenario::FieldPastTheTable => crash_of(|| {
            let tree = small_tree();
            let parent = value(
                &rows_over(&tree.elements),
                ElementPart::Parent,
                Position(10_000),
            );
            assert_eq!(parent, Cell::Flag(Flag::Lowered));
        }),
    };
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(shown.into_boxed_str())
}

/// What `work` crashed with: its panic's message, or nothing when it did not crash.
fn crash_of(work: impl FnOnce()) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default()
        },
        |()| String::new(),
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_are_ordered_by_their_keys"),
        word: Word::Succeeds,
        input: || Scenario::Sorted,
        expect: Expect::Value("[1, 2, 0]"),
    },
    Case {
        name: CaseName("the_rows_of_one_key_are_found_in_the_order"),
        word: Word::Succeeds,
        input: || Scenario::Hits,
        expect: Expect::Value("0..2"),
    },
    Case {
        name: CaseName("the_first_row_of_each_key_is_marked"),
        word: Word::Succeeds,
        input: || Scenario::Firsts,
        expect: Expect::Value("[true, true, false]"),
    },
    Case {
        name: CaseName("positions_are_counted_and_read_a_row_at_a_time"),
        word: Word::Succeeds,
        input: || Scenario::Rows,
        expect: Expect::Value("2 [4, 5, 6]"),
    },
    Case {
        name: CaseName("a_row_past_the_last_crashes_instead_of_reading_as_empty"),
        word: Word::Succeeds,
        input: || Scenario::RowPastTheLast,
        expect: Expect::Value("a row these positions hold"),
    },
    Case {
        name: CaseName(
            "a_field_of_a_position_past_the_table_crashes_instead_of_reading_as_the_root",
        ),
        word: Word::Succeeds,
        input: || Scenario::FieldPastTheTable,
        expect: Expect::Value("a position this table gave"),
    },
    Case {
        name: CaseName("a_join_or_a_walk_from_a_position_past_the_tree_crashes_the_root_is_placed"),
        word: Word::Succeeds,
        input: || Scenario::PlacePastTheTree,
        expect: Expect::Value("0 a position of the tree the rows were read from"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

fn raw_order() -> (
    std::sync::Arc<ryo_derived::domain::contract::types::PreparedDerived>,
    std::sync::Arc<ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl>,
    ryo_derived::domain::contract::types::DerivedPayload,
) {
    use ryo_derived::domain::contract::types::{
        BuildOutcome, BuildWorkspace, DerivedInputs, ViewRequest,
    };
    let port = std::sync::Arc::new(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    );
    let prepared = port.prepare(ViewRequest::RawKeyOrder { stride: 2 }, Vec::new());
    let BuildOutcome::Built(payload) = port
        .build(
            &prepared,
            DerivedInputs::RawWords(&keys().0),
            &mut BuildWorkspace::default(),
            None,
        )
        .unwrap()
    else {
        panic!("raw key order")
    };
    (prepared, port, payload)
}

use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::RowIndex};
use {crate::domain::table_model::types::Rows, ryo_values::domain::rows::errors::TableModelError};
/// Finite typed input adapter for pure semantic conformance, with no cache or scheduler.
pub(crate) struct SemanticRows<'f>(pub crate::domain::table_model::types::Rows<'f>);
impl ryo_derived::domain::contract::types::RowCells for SemanticRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::contract::types::BulkRows<'_>> {
        match &self.0 {
            Rows::Stored(source) => Some(ryo_derived::domain::contract::types::BulkRows::Stored(
                *source,
            )),
            Rows::Computed(table) => Some(
                ryo_derived::domain::contract::types::BulkRows::Computed(table),
            ),
            _ => None,
        }
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::contract::types::DerivedFault> {
        Ok(())
    }
    fn kinds(&self) -> &[CellKind] {
        crate::domain::table_model::service::kinds_of(&self.0)
    }
    fn count(&self) -> usize {
        crate::domain::table_model::service::count_of(&self.0).0
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Cell> {
        crate::domain::table_model::service::cell_at(&self.0, RowIndex(row), column)
    }
}
pub(crate) fn prepared_payload(
    request: ryo_derived::domain::contract::types::ViewRequest,
    rows: &Rows<'_>,
) -> Result<ryo_derived::domain::contract::types::DerivedPayload, TableModelError> {
    use ryo_derived::domain::contract::types::{
        BuildOutcome, BuildWorkspace, DerivedFault, DerivedInputs,
    };
    let port = std::sync::Arc::new(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    );
    let prepared = port.prepare(
        request,
        vec![crate::domain::table_model::service::kinds_of(rows).to_vec()],
    );
    let input = SemanticRows(rows.clone());
    match port.build(
        &prepared,
        DerivedInputs::Rows(&input),
        &mut BuildWorkspace::default(),
        None,
    ) {
        Ok(BuildOutcome::Built(payload)) => Ok(payload),
        Err(DerivedFault::NoSuchColumn { column }) => {
            Err(TableModelError::NoSuchColumn { index: column.0 })
        }
        other => panic!("unexpected semantic fixture result: {other:?}"),
    }
}
pub(crate) fn prepared_index(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<std::sync::Arc<KeyIndex>, TableModelError> {
    prepared_payload(
        ryo_derived::domain::contract::types::ViewRequest::Equality {
            columns: columns.to_vec(),
        },
        rows,
    )
    .map(|payload| {
        payload
            .share_equality()
            .expect("successful equality payload")
    })
}

pub(crate) fn prepared_order(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<Vec<RowIndex>, TableModelError> {
    use ryo_derived::domain::contract::types::{DerivedView, ViewRequest};
    let request = ViewRequest::PartitionOrder {
        keys: columns.to_vec(),
    };
    let payload = prepared_payload(request.clone(), rows)?;
    let port = std::sync::Arc::new(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    );
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
