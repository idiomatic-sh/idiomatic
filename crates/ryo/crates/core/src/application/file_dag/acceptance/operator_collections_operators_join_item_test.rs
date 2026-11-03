//! The `join` item's cases.
use crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture;
use crate::application::file_dag::feed::Feed;

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::relations::service::{node_id, to_bits};
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, FileId, Integer, KindBits, Offset, Position, Relation, Span,
    Value as Cell,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plans::types::QueryOperator;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::collections::operators::join::item::JoinItem;
use crate::domain::operator::contract::types::OperatorContract;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::{kinds_of, push_row, row_in};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as PlanInput, JoinColumn, LiteralText, Operand, QueryLiteral,
    Side, Test,
};
use ryo_values::domain::plans::types::Operation;

/// How many rows came out, or 1 when every expectation of the scenario held.
type Value = usize;
type Failure = OperatingError;
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);
const NUMBER_PAIR_KINDS: &[CellKind] = &[CellKind::Number, CellKind::Number];

fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

fn table_of(kinds: &[CellKind], rows: &[&[Cell]]) -> Table {
    let mut table = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    for row in rows {
        push_row(&mut table, row);
    }
    table
}

/// A join of `left_on` with `right_on`, answering the `output`, keeping what the `filter` admits, the right read through the `view`.
fn plan(
    left_on: &[u32],
    right_on: &[u32],
    output: &[(Side, u32)],
    filter: Vec<Comparison>,
    view: &[u32],
) -> Operation {
    let columns = |places: &[u32]| places.iter().map(|place| ColumnIndex(*place)).collect();
    Operation::Join {
        left: FROM,
        right: FROM,
        left_on: columns(left_on),
        right_on: columns(right_on),
        output: output
            .iter()
            .map(|(side, place)| JoinColumn {
                side: *side,
                column: ColumnIndex(*place),
            })
            .collect(),
        filter,
        view: columns(view),

        reach: None,
    }
}

fn made(
    operation: &Operation,
    rows: &[Rows<'_>],
    symbols: &mut Symbols,
    source: &[u8],
) -> Result<Table, OperatingError> {
    let mut out = Table::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    JoinItem.compute(operation, rows, &mut context, &mut out)?;
    Ok(out)
}

/// A root holding `tokens` tokens and a node with a token (its kind named when `named`); with one token, positions 0 to 3.
fn wide_tree(tokens: u32, named: bool) -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    {
        let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
        enter_node(&mut builder, KindBits(1), Offset(0));
        for _ in 0..tokens {
            token(&mut builder, KindBits(2), Offset(0), Offset(1));
        }
        enter_node(&mut builder, KindBits(3), Offset(1));
        token(&mut builder, KindBits(2), Offset(1), Offset(2));
        leave_node(&mut builder, Offset(2));
        leave_node(&mut builder, Offset(2));
    }
    if named {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(3),
            NameText("inner"),
        );
        index_kinds(&mut elements);
    }
    (elements, symbols)
}

fn tree_rows(elements: &Elements, form: TreeForm) -> Rows<'_> {
    Rows::Tree(TreeRows {
        elements,
        file: FileId(0),
        form,
    })
}

fn text_of(place: u32, text: &str) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: None,
    }
}

fn agreed(made: Result<Table, OperatingError>, expected: &[Cell]) -> Result<Value, OperatingError> {
    Ok(Value::from(made?.cells == expected))
}

/// Joins nodes of the file with the edges of its tree on the column of the edges, answering the parents found.
fn joins_the_tree(starts: &[u32], on: u32) -> Result<Table, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let mut left = table_of(&[CellKind::Node], &[]);
    for start in starts {
        push_row(&mut left, &[node(*start)]);
    }
    let output = [(Side::Right, 0), (Side::Right, 1)];
    let operation = plan(&[0], &[on], &output, Vec::new(), &[]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    made(&operation, &inputs, &mut symbols, &[])
}

fn a_node_joined_on_the_child_of_the_tree_finds_its_parent() -> Result<Value, OperatingError> {
    let parents = [node(2), node(3), node(0), node(1)];
    agreed(joins_the_tree(&[3, 1], 1), &parents)
}

fn a_node_joined_on_the_parent_of_the_tree_finds_its_children() -> Result<Value, OperatingError> {
    let children = [node(0), node(1), node(0), node(2)];
    agreed(joins_the_tree(&[0], 0), &children)
}

fn the_root_has_no_parent_and_a_leaf_no_children() -> Result<Value, OperatingError> {
    let (root, leaf) = (joins_the_tree(&[0], 1)?, joins_the_tree(&[3], 0)?);
    Ok(Value::from(root.cells.is_empty() && leaf.cells.is_empty()))
}

/// Joins nodes with the elements on their ids, keeping those whose text is `b`: answers the node and the span.
fn only_the_elements_a_filter_admits_are_joined_by_their_ids() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(1)], &[node(3)], &[node(2)]]);
    let output = [(Side::Left, 0), (Side::Right, 1)];
    let operation = plan(&[0], &[0], &output, vec![text_of(2, "b")], &[0, 4]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(3), stretch(1, 2)])
}

/// Joins the root with the edges whose child is the second of its parent's, child first: answers the children.
fn only_the_edges_a_filter_admits_are_joined() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, false);
    let left = table_of(&[CellKind::Node], &[&[node(0)]]);
    let second = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Number(Integer(1))),
        origin: None,
    };
    let operation = plan(&[0], &[1], &[(Side::Right, 0)], vec![second], &[1, 0]);
    let inputs = [Rows::Computed(&left), tree_rows(&elements, TreeForm::Edges)];
    agreed(made(&operation, &inputs, &mut symbols, b"abcd"), &[node(2)])
}

/// Joins every edge of a wide tree with the elements of a kind, by the child of the edge: answers the parent and the child.
fn the_edges_of_the_children_of_a_kind_are_found_by_the_kind() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let output = [(Side::Left, 0), (Side::Right, 0)];
    let operation = plan(&[1], &[0], &output, vec![text_of(1, "inner")], &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(0), node(41)])
}

/// Joins the edges of a wide tree with a few rows naming nodes, by the child of the edge: answers the children, in the order of the edges.
fn a_few_rows_leave_in_the_order_of_the_edges() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let number = |value| Cell::Number(Integer(value));
    let right = table_of(
        &[CellKind::Node, CellKind::Number],
        &[
            &[node(41), number(2)],
            &[node(5), number(1)],
            &[node(5), number(3)],
        ],
    );
    let output = [(Side::Left, 1), (Side::Right, 1)];
    let operation = plan(&[1], &[0], &output, Vec::new(), &[]);
    let inputs = [
        tree_rows(&elements, TreeForm::Edges),
        Rows::Computed(&right),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    let expected = [node(5), number(1), node(5), number(3), node(41), number(2)];
    agreed(joined, &expected)
}

/// Joins the stretch of the node at 41 of a wide tree, on the stretch alone, with the elements covering it.
fn the_elements_covering_a_stretch_are_found_by_it_alone() -> Result<Value, OperatingError> {
    let (elements, mut symbols) = wide_tree(40, true);
    let left = table_of(&[CellKind::Span], &[&[stretch(1, 2)]]);
    let operation = plan(&[0], &[4], &[(Side::Right, 0)], Vec::new(), &[]);
    let inputs = [
        Rows::Computed(&left),
        tree_rows(&elements, TreeForm::Elements),
    ];
    let joined = made(&operation, &inputs, &mut symbols, b"abcd");
    agreed(joined, &[node(41), node(42)])
}

/// Computes a join of two tables of numbers (the second is left out when it is `None`), answering how many rows came out.
fn counted(left: &Table, right: Option<&Table>) -> Result<Value, OperatingError> {
    let output = [(Side::Left, 1), (Side::Right, 1)];
    let operation = plan(&[0], &[0], &output, Vec::new(), &[]);
    let rows: Vec<Rows<'_>> = [Some(left), right.clone()]
        .into_iter()
        .flatten()
        .map(Rows::Computed)
        .collect();
    made(&operation, &rows, &mut Symbols::default(), &[]).map(|joined| joined.count.0)
}

fn joins_on_equal_keys() -> Result<Value, OperatingError> {
    let pairs = |rows: &[[i64; 2]]| {
        let mut table = table_of(&[CellKind::Number, CellKind::Number], &[]);
        for row in rows {
            push_row(&mut table, &row.map(|number| Cell::Number(Integer(number))));
        }
        table
    };
    let (left, right) = (
        pairs(&[[1, 10], [2, 20]]),
        pairs(&[[1, 100], [1, 200], [3, 300]]),
    );
    counted(&left, Some(&right))
}

fn joins_without_the_right_input() -> Result<Value, OperatingError> {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    counted(&left, None)
}

struct ControlledFeed<'a> {
    left: &'a Table,
    right: Rows<'a>,
    right_holds: usize,
    left_fails: bool,
    right_fails: bool,
}

impl<'f> Feed<'f> for ControlledFeed<'f> {
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        match which {
            InputPlace(0) => Ok(&self.left.kinds),
            InputPlace(1) => Ok(kinds_of(&self.right)),
            _ => Err(OperatingError::MissingInput {
                position: which.0 + 1,
            }),
        }
    }

    fn streams(&self, which: InputPlace) -> Supply {
        match which {
            InputPlace(0) => Supply::Streamed,
            _ => Supply::AtHand,
        }
    }

    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if which != InputPlace(0) {
            return Err(OperatingError::WrongOperation);
        }
        if self.left_fails {
            return Err(OperatingError::NotRun);
        }
        for position in 0..self.left.count.0 {
            let Some(row) = row_in(
                self.left,
                crate::domain::table_cells::types::RowIndex(position),
            ) else {
                continue;
            };
            emit.row(context, row)?;
        }
        Ok(())
    }

    fn found(&mut self, _which: InputPlace, _rows: RowCount) {}

    fn hold<'t>(
        &mut self,
        which: InputPlace,
        _context: &mut OperatorContext<'_>,
        _into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        if which != InputPlace(1) {
            return Err(OperatingError::WrongOperation);
        }
        self.right_holds += 1;
        if self.right_fails {
            return Err(OperatingError::NotRead);
        }
        Ok(self.right.clone())
    }
}

#[derive(Default)]
struct CollectedRows {
    rows: Vec<Vec<Cell>>,
    fails: bool,
}

impl Emit for CollectedRows {
    fn row(
        &mut self,
        _context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        if self.fails {
            return Err(OperatingError::NotRun);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}

fn fact_column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|value| to_bits(Cell::Number(Integer(*value))).0)
            .collect(),
    )
}

fn stream_lookup(
    room: &mut Room,
    left: &Table,
    right: Rows<'_>,
    left_fails: bool,
    right_fails: bool,
    emit_fails: bool,
) -> Result<(Vec<Vec<Cell>>, usize), OperatingError> {
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    stream_operation(
        &operation,
        room,
        left,
        right.clone(),
        left_fails,
        right_fails,
        emit_fails,
    )
}

/// Exercise the real streamed path for arbitrary key and output arrangements.
fn stream_operation(
    operation: &Operation,
    room: &mut Room,
    left: &Table,
    right: Rows<'_>,
    left_fails: bool,
    right_fails: bool,
    emit_fails: bool,
) -> Result<(Vec<Vec<Cell>>, usize), OperatingError> {
    stream_with_indexes(
        operation,
        room,
        left,
        right.clone(),
        (left_fails, right_fails, emit_fails),
        None,
    )
}

/// Exercise the actual streamed operator boundary with an optional file owner.
fn stream_with_indexes(
    operation: &Operation,
    room: &mut Room,
    left: &Table,
    right: Rows<'_>,
    failures: (bool, bool, bool),
    fixture: Option<&ResourceFixture>,
) -> Result<(Vec<Vec<Cell>>, usize), OperatingError> {
    stream_with_prepared_indexes(operation, room, left, right, failures, fixture, None)
}

fn stream_with_prepared_indexes(
    operation: &Operation,
    room: &mut Room,
    left: &Table,
    right: Rows<'_>,
    failures: (bool, bool, bool),
    fixture: Option<&ResourceFixture>,
    prepared: Option<&OperatorContract>,
) -> Result<(Vec<Vec<Cell>>, usize), OperatingError> {
    let (left_fails, right_fails, emit_fails) = failures;
    let mut feed = ControlledFeed {
        left,
        right: right.clone(),
        right_holds: 0,
        left_fails,
        right_fails,
    };
    let mut emitted = CollectedRows {
        fails: emit_fails,
        ..CollectedRows::default()
    };
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let inputs = [Rows::Computed(left), right.clone()];
    if let Some(fixture) = fixture {
        if let Some(prepared) = prepared {
            fixture.stream_prepared(
                operation,
                prepared,
                &inputs,
                &mut feed,
                &mut context,
                room,
                &mut emitted,
            )?;
        } else {
            fixture.stream(
                operation,
                &inputs,
                &mut feed,
                &mut context,
                room,
                &mut emitted,
            )?;
        }
    } else {
        crate::application::file_dag::testing::stream_with_inputs(
            &JoinItem,
            operation,
            &inputs,
            &mut feed,
            &mut context,
            room,
            &mut emitted,
        )?;
    }
    Ok((emitted.rows, feed.right_holds))
}

fn empty_left_does_not_materialize_stored_right() -> Result<Value, OperatingError> {
    let left = table_of(&[CellKind::Number], &[]);
    for size in [1_i64, 1024] {
        let values: Vec<_> = (0..size).collect();
        let columns = [fact_column(&values), fact_column(&values)];
        let right = Rows::Stored(ColumnsView {
            kinds: NUMBER_PAIR_KINDS,
            columns: &columns,
        });
        let mut room = Room::default();
        let (rows, holds) = stream_lookup(&mut room, &left, right.clone(), false, false, false)?;
        if !rows.is_empty() || holds != 1 || room.scratch.observation.index_cells != 0 {
            return Ok(0);
        }
    }
    Ok(1)
}

fn empty_left_still_refuses_unread_right() -> Result<Value, OperatingError> {
    let left = table_of(&[CellKind::Number], &[]);
    let right = table_of(&[CellKind::Number, CellKind::Number], &[]);
    let mut room = Room::default();
    let _held = stream_lookup(&mut room, &left, Rows::Computed(&right), false, true, false)?;
    Ok(1)
}

fn duplicate_unsorted_right_rows_keep_their_order() -> Result<Value, OperatingError> {
    let number = |value| Cell::Number(Integer(value));
    let left = table_of(&[CellKind::Number], &[&[number(2)]]);
    let columns = [fact_column(&[2, 1, 2, 2]), fact_column(&[20, 10, 21, 22])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let mut room = Room::default();
    let (rows, holds) = stream_lookup(&mut room, &left, right.clone(), false, false, false)?;
    let expected = [vec![number(20)], vec![number(21)], vec![number(22)]];
    Ok(Value::from(rows == expected && holds == 1))
}

fn a_reused_room_skips_then_reacquires_its_right_index() -> Result<Value, OperatingError> {
    let number = |value| Cell::Number(Integer(value));
    let first_left = table_of(&[CellKind::Number], &[&[number(1)]]);
    let empty_left = table_of(&[CellKind::Number], &[]);
    let last_left = table_of(&[CellKind::Number], &[&[number(1)]]);
    let first_columns = [fact_column(&[1, 1]), fact_column(&[11, 12])];
    let middle_columns = [fact_column(&[1]), fact_column(&[99])];
    let last_columns = [fact_column(&[1]), fact_column(&[31])];
    let first_right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &first_columns,
    });
    let middle_right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &middle_columns,
    });
    let last_right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &last_columns,
    });
    let mut room = Room::default();
    let first = stream_lookup(&mut room, &first_left, first_right, false, false, false)?.0;
    assert_eq!(room.scratch.observation.index_rows, 2);
    assert_eq!(room.scratch.observation.index_cells, 0);
    assert!(room.scratch.observation.local_resources_released);
    let skipped = stream_lookup(&mut room, &empty_left, middle_right, false, false, false)?.0;
    let skipped_acquisition = room.scratch.observation.index_rows == 0
        && room.scratch.observation.index_cells == 0
        && room.scratch.observation.local_resources_released;
    let last = stream_lookup(&mut room, &last_left, last_right, false, false, false)?.0;
    Ok(Value::from(
        first == [vec![number(11)], vec![number(12)]]
            && skipped.is_empty()
            && skipped_acquisition
            && last == [vec![number(31)]],
    ))
}

fn a_left_failure_before_its_first_row_does_not_materialize_the_right()
-> Result<Value, OperatingError> {
    let left = table_of(&[CellKind::Number], &[]);
    let values: Vec<_> = (0..1024).collect();
    let columns = [fact_column(&values), fact_column(&values)];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let mut room = Room::default();
    let result = stream_lookup(&mut room, &left, right.clone(), true, false, false);
    if room.scratch.observation.index_rows == 0 && room.scratch.observation.index_cells == 0 {
        result.map(|_| 1)
    } else {
        Ok(0)
    }
}

fn an_emit_failure_propagates_after_the_right_is_indexed() -> Result<Value, OperatingError> {
    let number = Cell::Number(Integer(1));
    let left = table_of(&[CellKind::Number], &[&[number]]);
    let columns = [fact_column(&[1]), fact_column(&[10])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let mut room = Room::default();
    let result = stream_lookup(&mut room, &left, right.clone(), false, false, true);
    if room.scratch.observation.index_rows == 1 {
        result.map(|_| 1)
    } else {
        Ok(0)
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_keys_combine"),
        word: Word::Succeeds,
        input: || joins_on_equal_keys,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_node_joined_on_the_child_of_the_tree_finds_its_parent"),
        word: Word::Succeeds,
        input: || a_node_joined_on_the_child_of_the_tree_finds_its_parent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_joined_on_the_parent_of_the_tree_finds_its_children"),
        word: Word::Succeeds,
        input: || a_node_joined_on_the_parent_of_the_tree_finds_its_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_root_has_no_parent_and_a_leaf_no_children"),
        word: Word::Succeeds,
        input: || the_root_has_no_parent_and_a_leaf_no_children,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_elements_a_filter_admits_are_joined_by_their_ids"),
        word: Word::Succeeds,
        input: || only_the_elements_a_filter_admits_are_joined_by_their_ids,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_edges_a_filter_admits_are_joined_in_the_view_they_are_asked_in"),
        word: Word::Succeeds,
        input: || only_the_edges_a_filter_admits_are_joined,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_edges_of_the_children_of_a_kind_are_found_by_the_kind"),
        word: Word::Succeeds,
        input: || the_edges_of_the_children_of_a_kind_are_found_by_the_kind,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_elements_covering_a_stretch_are_found_by_it_alone"),
        word: Word::Succeeds,
        input: || the_elements_covering_a_stretch_are_found_by_it_alone,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_few_rows_find_their_edges_and_leave_in_the_order_of_the_edges"),
        word: Word::Succeeds,
        input: || a_few_rows_leave_in_the_order_of_the_edges,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || joins_without_the_right_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
    Case {
        name: CaseName("an_empty_streamed_left_leaves_the_stored_right_unmaterialized"),
        word: Word::Succeeds,
        input: || empty_left_does_not_materialize_stored_right,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_empty_left_still_holds_and_refuses_an_unread_right"),
        word: Word::Refuses,
        input: || empty_left_still_refuses_unread_right,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::NotRead))),
    },
    Case {
        name: CaseName("unsorted_duplicate_stored_rows_keep_their_order"),
        word: Word::Succeeds,
        input: || duplicate_unsorted_right_rows_keep_their_order,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_reused_room_skips_empty_then_uses_the_next_right"),
        word: Word::Succeeds,
        input: || a_reused_room_skips_then_reacquires_its_right_index,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_left_failure_before_the_first_row_does_not_materialize_the_right"),
        word: Word::Refuses,
        input: || a_left_failure_before_its_first_row_does_not_materialize_the_right,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::NotRun))),
    },
    Case {
        name: CaseName("an_emit_failure_after_the_first_row_propagates"),
        word: Word::Refuses,
        input: || an_emit_failure_propagates_after_the_right_is_indexed,
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::NotRun))),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(QueryOperator::Join);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Compare computed and streamed paths for one key schema while reusing the same operator room.
fn assert_stored_join_shape(
    operation: &Operation,
    left: &Table,
    right: Rows<'_>,
    computed: &Table,
    room: &mut Room,
) {
    let expected = made(
        operation,
        &[Rows::Computed(left), Rows::Computed(computed)],
        &mut Symbols::default(),
        &[],
    )
    .expect("computed join");
    let actual = made(
        operation,
        &[Rows::Computed(left), right.clone()],
        &mut Symbols::default(),
        &[],
    )
    .expect("stored join");
    assert_eq!(actual, expected);
    let expected_rows = (0..expected.count.0)
        .map(|at| {
            row_in(&expected, crate::domain::table_cells::types::RowIndex(at))
                .expect("expected row")
                .to_vec()
        })
        .collect::<Vec<_>>();
    let stored_rows = stream_operation(operation, room, left, right.clone(), false, false, false)
        .expect("streamed stored")
        .0;
    assert_eq!(stored_rows, expected_rows);
    assert!(
        room.scratch.observation.index_cells == 0,
        "only a matched RHS row is decoded, never the full stored table"
    );
    assert!(room.scratch.observation.index_capacity == 0);
    let computed_rows = stream_operation(
        operation,
        room,
        left,
        Rows::Computed(computed),
        false,
        false,
        false,
    )
    .expect("streamed computed")
    .0;
    assert_eq!(computed_rows, expected_rows);
    let empty = table_of(&left.kinds, &[]);
    assert!(
        stream_operation(operation, room, &empty, right.clone(), false, false, false)
            .expect("empty left")
            .0
            .is_empty()
    );
}

#[test]
fn stored_right_joins_match_computed_order_for_composite_reordered_and_empty_keys() {
    use crate::domain::table_model::service::materialize;
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag, CellKind::Number];
    let columns = [
        fact_column(&[1, 2, 1, 3, 1]),
        FactColumn(vec![2, 0, 1, 1, 0]),
        fact_column(&[10, 20, 11, 30, 12]),
    ];
    let right = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns: &columns,
    });
    let mut computed = Table::default();
    materialize(&right, &mut computed, &mut Vec::new());
    let number = |value| Cell::Number(Integer(value));
    let left = table_of(
        &[CellKind::Number, CellKind::Flag],
        &[
            &[
                number(1),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Raised),
            ],
            &[
                number(1),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Lowered),
            ],
            &[
                number(1),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Raised),
            ],
            &[
                number(9),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Raised),
            ],
        ],
    );
    let mut room = Room::default();
    for (left_on, right_on) in [
        (vec![0], vec![0]),
        (vec![1, 0], vec![1, 0]),
        (vec![0, 1], vec![0, 1]),
        (Vec::new(), Vec::new()),
        (vec![0], vec![1]),
    ] {
        let operation = plan(
            &left_on,
            &right_on,
            &[
                (Side::Right, 2),
                (Side::Left, 0),
                (Side::Right, 1),
                (Side::Right, 0),
                (Side::Right, 2),
            ],
            Vec::new(),
            &[],
        );
        assert_stored_join_shape(&operation, &left, right.clone(), &computed, &mut room);
    }
}

#[test]
fn compact_right_joins_preserve_duplicate_positions_reordered_keys_and_typed_flags() {
    use crate::domain::table_model::service::materialize;
    use crate::domain::table_model::types::{SelectedRows, StoredSelection};
    use std::sync::Arc;
    let columns = [
        fact_column(&[1, 2, 1, 3, 1]),
        FactColumn(vec![2, 0, 1, 1, 0]),
        fact_column(&[10, 20, 11, 30, 12]),
    ];
    let right = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: &[CellKind::Number, CellKind::Flag, CellKind::Number],
            columns: &columns,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![4, 1, 0, 4]),
            columns: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
            kinds: vec![CellKind::Number, CellKind::Number, CellKind::Flag],
            identity: Arc::new(Default::default()),
        }),
    });
    let left = table_of(
        &[CellKind::Number, CellKind::Flag],
        &[
            &[
                Cell::Number(Integer(1)),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Lowered),
            ],
            &[
                Cell::Number(Integer(1)),
                Cell::Flag(ryo_language::domain::relations::types::Flag::Raised),
            ],
        ],
    );
    let mut computed = Table::default();
    materialize(&right, &mut computed, &mut Vec::new());
    let mut room = Room::default();
    for (left_on, right_on) in [
        (vec![0, 1], vec![1, 2]),
        (vec![1, 0], vec![2, 1]),
        (vec![], vec![]),
    ] {
        let operation = plan(
            &left_on,
            &right_on,
            &[(Side::Right, 0), (Side::Right, 0), (Side::Left, 0)],
            Vec::new(),
            &[],
        );
        assert_stored_join_shape(&operation, &left, right.clone(), &computed, &mut room);
    }
}

#[test]
fn malformed_stored_views_keep_the_original_materializing_fallback() {
    use crate::domain::table_model::service::materialize;
    let columns = [fact_column(&[1, 1, 1]), fact_column(&[10, 11])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let mut computed = Table::default();
    materialize(&right, &mut computed, &mut Vec::new());
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    assert_eq!(
        made(
            &operation,
            &[Rows::Computed(&left), right.clone()],
            &mut Symbols::default(),
            &[]
        ),
        made(
            &operation,
            &[Rows::Computed(&left), Rows::Computed(&computed)],
            &mut Symbols::default(),
            &[]
        )
    );
    let mut room = Room::default();
    assert_eq!(
        stream_operation(
            &operation,
            &mut room,
            &left,
            right.clone(),
            false,
            false,
            false
        )
        .expect("stored")
        .0,
        stream_operation(
            &operation,
            &mut room,
            &left,
            Rows::Computed(&computed),
            false,
            false,
            false
        )
        .expect("computed")
        .0
    );
}

#[test]
fn stored_right_key_errors_match_computed_before_any_emission() {
    use crate::domain::table_model::service::materialize;
    let columns = [fact_column(&[1]), fact_column(&[10])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let mut computed = Table::default();
    materialize(&right, &mut computed, &mut Vec::new());
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    for (left_on, right_on) in [(vec![9], vec![0]), (vec![0], vec![9])] {
        let operation = plan(&left_on, &right_on, &[(Side::Right, 1)], Vec::new(), &[]);
        let expected = made(
            &operation,
            &[Rows::Computed(&left), Rows::Computed(&computed)],
            &mut Symbols::default(),
            &[],
        );
        assert!(expected.is_err());
        assert_eq!(
            made(
                &operation,
                &[Rows::Computed(&left), right.clone()],
                &mut Symbols::default(),
                &[]
            ),
            expected
        );
        assert_eq!(
            stream_operation(
                &operation,
                &mut Room::default(),
                &left,
                right.clone(),
                false,
                false,
                false
            ),
            stream_operation(
                &operation,
                &mut Room::default(),
                &left,
                Rows::Computed(&computed),
                false,
                false,
                false
            )
        );
    }
}

/// Exercise the held-input operator with an audited file owner and no stream fixture.
fn made_with_indexes(
    operation: &Operation,
    left: Rows<'_>,
    right: Rows<'_>,
    fixture: &crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture,
) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut out = Table::default();
    fixture.collect(operation, &[left, right], &mut context, &mut out)?;
    Ok(out)
}

/// A valid Function population with repeated typed keys and distinct payloads.
fn sparse_function_columns(count: usize) -> [FactColumn; 4] {
    let mut columns = std::array::from_fn(|_| FactColumn::default());
    for place in 0..count {
        let row = [
            node((place % 8 + 1) as u32),
            Cell::Symbol(ryo_language::domain::terms::types::Symbol(1)),
            Cell::File(FileId((place % 2) as u32)),
            stretch(place as u32, place as u32 + 1),
        ];
        for (column, value) in columns.iter_mut().zip(row) {
            column.0.push(to_bits(value).0);
        }
    }
    columns
}

#[test]
fn sparse_left_matches_computed_bag_order_for_composite_typed_duplicate_keys() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    use crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture;
    use crate::domain::table_model::service::materialize;
    let columns = sparse_function_columns(256);
    let left = Rows::Stored(ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::Function),
        columns: &columns,
    });
    let right = table_of(
        &[CellKind::Node, CellKind::File, CellKind::Number],
        &[
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(30))],
            &[node(1), Cell::File(FileId(0)), Cell::Number(Integer(10))],
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(31))],
            &[node(7), Cell::File(FileId(1)), Cell::Number(Integer(70))],
        ],
    );
    let operation = plan(
        &[2, 0],
        &[1, 0],
        &[(Side::Left, 3), (Side::Right, 2), (Side::Left, 0)],
        Vec::new(),
        &[],
    );
    let mut materialized = Table::default();
    materialize(&left, &mut materialized, &mut Vec::new());
    let expected = made(
        &operation,
        &[Rows::Computed(&materialized), Rows::Computed(&right)],
        &mut Symbols::default(),
        &[],
    )
    .expect("ordinary join");
    let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("complete file");
    let actual = made_with_indexes(&operation, left.clone(), Rows::Computed(&right), &fixture)
        .expect("sparse join");
    assert_eq!(actual, expected);
    assert_eq!(fixture.snapshot().builds, 1);
    assert_eq!(fixture.snapshot().entries, 1);
    let again = made_with_indexes(&operation, left, Rows::Computed(&right), &fixture)
        .expect("shared sparse join");
    assert_eq!(again, expected);
    assert_eq!(fixture.snapshot().reuses, 1);
}

#[test]
fn sparse_left_falls_back_for_partial_or_unretained_sources_and_empty_results() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    use crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture;
    use crate::domain::coverage::types::{Lacking, Partial};
    use ryo_values::domain::stored_facts::types::PartialCause;
    let columns = sparse_function_columns(256);
    let left = Rows::Stored(ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::Function),
        columns: &columns,
    });
    let right = table_of(&[CellKind::Node], &[&[node(99)]]);
    let operation = plan(&[0], &[0], &[(Side::Left, 3)], Vec::new(), &[]);
    let partial = ResourceFixture::new(
        &[Relation::Function],
        &[Partial {
            relation: Relation::Function,
            cause: PartialCause("incomplete fixture".into()),
            lacking: Lacking::Any,
            at: None,
        }],
        LIMIT,
    )
    .expect("file owner with partial facts");
    assert!(
        made_with_indexes(&operation, left.clone(), Rows::Computed(&right), &partial)
            .expect("partial fallback")
            .cells
            .is_empty()
    );
    assert_eq!(partial.snapshot().entries, 0);
    let limited = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("complete file");
    limited
        .owner()
        .derived
        .set_optional_limit(0)
        .expect("refuse optional retention");
    assert!(
        made_with_indexes(&operation, left, Rows::Computed(&right), &limited)
            .expect("budget fallback")
            .cells
            .is_empty()
    );
    assert_eq!(limited.snapshot().entries, 0);
}

#[test]
fn sparse_left_dense_pairs_fall_back_before_emitting_and_keep_bag_order() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    use crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture;
    use crate::domain::table_model::service::materialize;
    let mut columns = sparse_function_columns(4_097);
    columns[0].0.fill(to_bits(node(1)).0);
    let left = Rows::Stored(ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::Function),
        columns: &columns,
    });
    let mut right = Table {
        kinds: vec![CellKind::Node, CellKind::Number],
        ..Table::default()
    };
    for place in 0..16 {
        push_row(&mut right, &[node(1), Cell::Number(Integer(place))]);
    }
    let operation = plan(
        &[0],
        &[0],
        &[(Side::Left, 3), (Side::Right, 1)],
        Vec::new(),
        &[],
    );
    let mut computed_left = Table::default();
    materialize(&left, &mut computed_left, &mut Vec::new());
    let expected = made(
        &operation,
        &[Rows::Computed(&computed_left), Rows::Computed(&right)],
        &mut Symbols::default(),
        &[],
    )
    .expect("ordinary dense join");
    let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("complete file");
    let out = made_with_indexes(&operation, left, Rows::Computed(&right), &fixture)
        .expect("dense fallback");
    assert_eq!(out, expected);
    assert_eq!(
        fixture.snapshot().builds,
        1,
        "the sparse index was attempted"
    );
    assert_eq!(out.count.0, 4_097 * 16);
    assert_eq!(
        row_in(&out, crate::domain::table_cells::types::RowIndex(0)),
        Some(&[stretch(0, 1), Cell::Number(Integer(0))][..])
    );
    assert_eq!(
        row_in(
            &out,
            crate::domain::table_cells::types::RowIndex(out.count.0 - 1)
        ),
        Some(&[stretch(4_096, 4_097), Cell::Number(Integer(15))][..])
    );
}

/// Capture only the opt-in aggregate target, without adding a test dependency.
#[derive(Clone, Default)]
struct JoinDiagnostics {
    /// Recorded fields of each completed invocation, shared with the assertion scope.
    events: std::sync::Arc<std::sync::Mutex<Vec<std::collections::BTreeMap<String, String>>>>,
}

impl tracing::Subscriber for JoinDiagnostics {
    fn register_callsite(
        &self,
        _: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        // Other parallel tests use the no-op dispatcher. Never cache their disabled
        // decision for this thread-local capture, or this capture for their tests.
        tracing::subscriber::Interest::sometimes()
    }
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "ryo::diagnostic::stored_join"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = JoinDiagnosticFields::default();
        event.record(&mut fields);
        self.events
            .lock()
            .expect("diagnostic events")
            .push(fields.0);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// Convert the aggregate event's typed fields into independent expected text.
#[derive(Default)]
struct JoinDiagnosticFields(std::collections::BTreeMap<String, String>);
impl tracing::field::Visit for JoinDiagnosticFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _previous = self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let _previous = self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

/// Tracing callsite interest is process-global: parallel no-op dispatchers can race
/// first registration. Run this one observer contract in a fresh process while the
/// ordinary suite remains parallel; its four assertions still execute unchanged.
fn diagnostic_contract_in_child() -> bool {
    const CHILD: &str = "RYO_JOIN_DIAGNOSTIC_CONTRACT_CHILD";
    if std::env::var_os(CHILD).as_deref() == Some(std::ffi::OsStr::new("1")) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args(["--exact", "application::file_dag::acceptance::operator_collections_operators_join_item_test::stored_join_diagnostics_emit_one_aggregate_including_empty_and_failed_invocations", "--test-threads=1"])
        .env(CHILD, "1")
        .output().expect("isolated observer contract process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "observer child must execute its actual contract: status={:?}, stdout={stdout}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn stored_join_diagnostics_emit_one_aggregate_including_empty_and_failed_invocations() {
    if diagnostic_contract_in_child() {
        return;
    }
    let diagnostics = JoinDiagnostics::default();
    let captured = (diagnostics.events).clone();
    let columns = [fact_column(&[1, 1, 2]), fact_column(&[10, 11, 20])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(
        &[CellKind::Number],
        &[
            &[Cell::Number(Integer(1))],
            &[Cell::Number(Integer(1))],
            &[Cell::Number(Integer(9))],
        ],
    );
    let empty = table_of(&left.kinds, &[]);
    tracing::subscriber::with_default(diagnostics, || {
        let mut room = Room::default();
        assert_eq!(
            stream_lookup(&mut room, &left, right.clone(), false, false, false)
                .expect("stored join")
                .0
                .len(),
            4
        );
        assert!(
            stream_lookup(&mut room, &empty, right.clone(), false, false, false)
                .expect("empty")
                .0
                .is_empty()
        );
        let _emit_refusal = stream_lookup(&mut room, &left, right.clone(), false, false, true)
            .expect_err("downstream refusal must propagate");
        let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
        assert_eq!(
            made(
                &operation,
                &[Rows::Computed(&left), right.clone()],
                &mut Symbols::default(),
                &[]
            )
            .expect("computed execution")
            .count
            .0,
            4
        );
    });
    let events = captured.lock().expect("events");
    assert_eq!(
        events.len(),
        4,
        "one event per invocation, never one per probe or hit: {events:?}"
    );
    for (event, (builds, probes, hits, complete)) in events.iter().zip([
        (1_u64, 3_u64, 4_u64, true),
        (0, 0, 0, true),
        (1, 1, 1, false),
        (1, 3, 4, true),
    ]) {
        for (field, expected) in [
            ("rhs_rows", "3".to_owned()),
            ("rhs_width", "2".to_owned()),
            ("key_width", "1".to_owned()),
            ("builds", builds.to_string()),
            ("probes", probes.to_string()),
            ("hits", hits.to_string()),
            ("path", "stored-key".to_owned()),
            ("complete", complete.to_string()),
        ] {
            assert_eq!(event.get(field), Some(&expected), "field {field}");
        }
    }
}

#[test]
fn separate_rule_rooms_reuse_one_index_and_still_read_the_provider_before_hits() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    let columns = [fact_column(&[2, 1, 2, 2]), fact_column(&[20, 10, 21, 22])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(2))]]);
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    let fixture =
        ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("physical generation");
    let first = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    )
    .expect("first rule");
    let second = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    )
    .expect("second rule");
    assert_eq!(
        first.0,
        vec![
            vec![Cell::Number(Integer(20))],
            vec![Cell::Number(Integer(21))],
            vec![Cell::Number(Integer(22))]
        ]
    );
    assert_eq!(second, first);
    assert_eq!(
        (fixture.snapshot().builds, fixture.snapshot().reuses),
        (1, 1)
    );
    assert_eq!(
        first.1, 1,
        "a shared hit still performs the normal right hold"
    );
    let prior_reuses = fixture.snapshot().reuses;
    let _fixture_refusal = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        right.clone(),
        (false, true, false),
        Some(&fixture),
    )
    .expect_err("fixture refusal must propagate");
    assert_eq!(
        fixture.snapshot().reuses,
        prior_reuses,
        "provider failure is not hidden by a previously cached index"
    );
    let empty = table_of(&left.kinds, &[]);
    assert!(
        stream_with_indexes(
            &operation,
            &mut Room::default(),
            &empty,
            right.clone(),
            (false, false, false),
            Some(&fixture)
        )
        .expect("prepared empty-left join")
        .0
        .is_empty()
    );
    assert_eq!(fixture.snapshot().reuses, prior_reuses);
    // Request preparation records invalid metadata, but raises it only when a row needs the key.
    let invalid = plan(&[0], &[99], &[(Side::Right, 1)], Vec::new(), &[]);
    let invalid_empty = stream_with_indexes(
        &invalid,
        &mut Room::default(),
        &empty,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    );
    assert!(
        invalid_empty
            .expect("no row demands the invalid key")
            .0
            .is_empty()
    );
    assert_eq!(fixture.snapshot().reuses, prior_reuses);
    let invalid_prepared = stream_with_indexes(
        &invalid,
        &mut Room::default(),
        &left,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    );
    assert!(
        matches!(
            invalid_prepared,
            Err(OperatingError::Table(
                crate::domain::table_model::errors::TableModelError::NoSuchColumn { index: 99 }
            ))
        ),
        "prepared invalid join answered {invalid_prepared:?}"
    );
    assert_eq!(fixture.snapshot().reuses, prior_reuses);
    // The unprepared primitive retains its historical lazy empty-input behavior.
    assert!(
        stream_with_indexes(
            &invalid,
            &mut Room::default(),
            &empty,
            right.clone(),
            (false, false, false),
            None
        )
        .expect("unprepared empty left does not validate keys")
        .0
        .is_empty()
    );
    assert_eq!(fixture.snapshot().reuses, prior_reuses);
}

#[test]
fn cached_emit_error_releases_invocation_handle_and_next_generation_uses_new_rows() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    let columns = [fact_column(&[1]), fact_column(&[10])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    let schemas = [
        InputSchema {
            kinds: &left.kinds,
            tree: None,
        },
        InputSchema {
            kinds: NUMBER_PAIR_KINDS,
            tree: None,
        },
    ];
    let prepared = crate::application::file_dag::derived::prepare::operator(
        crate::domain::operator::service::registered(crate::domain::features::service::kind_of(
            &operation,
        )),
        &operation,
        &schemas,
        &[],
    )
    .expect("prepare one actual join contract for both invocations");
    let weak = {
        let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("generation");
        let warmed = stream_with_prepared_indexes(
            &operation,
            &mut Room::default(),
            &left,
            right.clone(),
            (false, false, false),
            Some(&fixture),
            Some(&prepared),
        )
        .expect("owner retains a reusable index");
        assert_eq!(warmed.0, vec![vec![Cell::Number(Integer(10))]]);
        assert!(fixture.no_invocation_index_handles());
        assert_eq!(fixture.shared_index_backings(), 1);
        let owner_refs = fixture.first_index_owners().expect("cached entry");
        let _fixture_refusal = stream_with_prepared_indexes(
            &operation,
            &mut Room::default(),
            &left,
            right.clone(),
            (false, false, true),
            Some(&fixture),
            Some(&prepared),
        )
        .expect_err("fixture refusal must propagate");
        assert!(
            fixture.no_invocation_index_handles(),
            "failed operator retains no escaped handle"
        );
        assert_eq!(fixture.shared_index_backings(), 1);
        assert_eq!(fixture.first_index_owners(), Some(owner_refs));
        let weak = fixture.first_index_weak().expect("cached entry");
        fixture.owner().derived.clear();
        assert!(
            weak.upgrade().is_none(),
            "clearing the owner releases the shared index"
        );
        weak
    };
    assert!(weak.upgrade().is_none());
    let new_columns = [fact_column(&[1]), fact_column(&[99])];
    let changed = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &new_columns,
    });
    let next =
        ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("same FileId, new snapshot");
    let rows = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        changed,
        (false, false, false),
        Some(&next),
    )
    .expect("new generation")
    .0;
    assert_eq!(rows, vec![vec![Cell::Number(Integer(99))]]);
    assert_eq!((next.snapshot().builds, next.snapshot().reuses), (1, 0));
}

/// Emit into a nested join while the outer invocation owns its shared immutable index handle.
struct NestedStoredJoin<'a> {
    /// The same audited physical population used by both invocations.
    right: Rows<'a>,
    fixture: &'a ResourceFixture,
    /// Collected nested results, retaining each duplicate.
    rows: Vec<Vec<Cell>>,
}
impl Emit for NestedStoredJoin<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        let left = table_of(&[CellKind::Number], &[row]);
        let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
        let mut output = Table::default();
        self.fixture.collect(
            &operation,
            &[Rows::Computed(&left), self.right.clone()],
            context,
            &mut output,
        )?;
        for at in 0..output.count.0 {
            self.rows.push(
                row_in(&output, crate::domain::table_cells::types::RowIndex(at))
                    .expect("nested output")
                    .to_vec(),
            );
        }
        Ok(())
    }
}

#[test]
fn shared_index_handle_releases_mutable_cache_borrow_before_nested_emission() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    let columns = [fact_column(&[2, 1, 2, 2]), fact_column(&[20, 10, 21, 22])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(2))]]);
    let operation = plan(&[0], &[0], &[(Side::Right, 0)], Vec::new(), &[]);
    let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("generation");
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut feed = ControlledFeed {
        left: &left,
        right: right.clone(),
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    let mut emitted = NestedStoredJoin {
        right: right.clone(),
        fixture: &fixture,
        rows: Vec::new(),
    };
    fixture
        .stream(
            &operation,
            &[Rows::Computed(&left), right.clone()],
            &mut feed,
            &mut context,
            &mut Room::default(),
            &mut emitted,
        )
        .expect("nested joins");
    let expected = [
        vec![Cell::Number(Integer(20))],
        vec![Cell::Number(Integer(21))],
        vec![Cell::Number(Integer(22))],
    ];
    assert_eq!(
        emitted.rows,
        expected.iter().cloned().cycle().take(9).collect::<Vec<_>>()
    );
    assert_eq!(
        (fixture.snapshot().builds, fixture.snapshot().reuses),
        (1, 3)
    );
}

#[test]
fn malformed_views_and_failed_upstream_still_use_original_paths_with_a_file_cache() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    let columns = [fact_column(&[1, 1, 1]), fact_column(&[10, 11])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("generation");
    let actual = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    )
    .expect("fallback")
    .0;
    assert_eq!(
        actual,
        vec![
            vec![Cell::Number(Integer(10))],
            vec![Cell::Number(Integer(11))]
        ]
    );
    assert!(
        fixture.snapshot().entries == 0,
        "malformed views are never admitted"
    );
    let complete_columns = [fact_column(&[1]), fact_column(&[99])];
    let complete = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &complete_columns,
    });
    let _fixture_refusal = stream_with_indexes(
        &operation,
        &mut Room::default(),
        &left,
        complete,
        (true, false, false),
        Some(&fixture),
    )
    .expect_err("fixture refusal must propagate");
    assert_eq!(
        fixture.snapshot().builds,
        0,
        "upstream error arrives before the lazy index build"
    );
}

#[test]
fn demand_reach_stream_preserves_lazy_holds_refusals_and_room_reuse() {
    use crate::application::file_dag::derived::indexes::LIMIT;
    use ryo_values::domain::plan_parts::types::ReachLookup;
    let columns = [fact_column(&[1, 1, 9, 2]), fact_column(&[9, 2, 1, 3])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let left = table_of(
        &[CellKind::Number],
        &[&[Cell::Number(Integer(1))], &[Cell::Number(Integer(1))]],
    );
    let empty = table_of(&[CellKind::Number], &[]);
    let mut operation = plan(&[0], &[0], &[(Side::Right, 1)], Vec::new(), &[]);
    if let Operation::Join { reach, .. } = &mut operation {
        *reach = Some(ReachLookup {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![],
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
        });
    }
    let fixture = ResourceFixture::new(&[Relation::Function], &[], LIMIT).expect("cache");
    let mut room = Room::default();
    let (nothing, holds) = stream_with_indexes(
        &operation,
        &mut room,
        &empty,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    )
    .expect("empty left");
    assert!(nothing.is_empty());
    assert_eq!(holds, 1);
    assert_eq!(fixture.snapshot().graph_builds, 0);
    assert!(matches!(
        stream_with_indexes(
            &operation,
            &mut room,
            &empty,
            right.clone(),
            (false, true, false),
            Some(&fixture)
        ),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        stream_with_indexes(
            &operation,
            &mut room,
            &left,
            right.clone(),
            (true, false, false),
            Some(&fixture)
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(fixture.snapshot().graph_builds, 0);
    assert!(matches!(
        stream_with_indexes(
            &operation,
            &mut room,
            &left,
            right.clone(),
            (false, false, true),
            Some(&fixture)
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(fixture.snapshot().graph_builds, 1);
    let (actual, holds) = stream_with_indexes(
        &operation,
        &mut room,
        &left,
        right.clone(),
        (false, false, false),
        Some(&fixture),
    )
    .expect("reused after emit failure");
    assert_eq!(holds, 1);
    let once = [1, 2, 3, 9].map(|value| vec![Cell::Number(Integer(value))]);
    let expected: Vec<_> = once.iter().chain(once.iter()).cloned().collect();
    assert_eq!(actual, expected);
    assert_eq!(fixture.snapshot().graph_builds, 1);
    assert!(fixture.snapshot().graph_reuses > 0);
    // A previous cache hit never suppresses a subsequent provider refusal.
    assert!(matches!(
        stream_with_indexes(
            &operation,
            &mut room,
            &left,
            right,
            (false, true, false),
            Some(&fixture)
        ),
        Err(OperatingError::NotRead)
    ));
    assert!(room.scratch.observation.traversal_queue_len == 0);
}
