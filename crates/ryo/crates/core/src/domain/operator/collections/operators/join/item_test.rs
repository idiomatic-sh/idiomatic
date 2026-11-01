//! The `join` item's cases.

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

use super::super::interface::OperatorsItem as _;
use super::item::JoinItem;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
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
        bindings: &[],
        symbols,
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
    let rows: Vec<Rows<'_>> = [Some(left), right]
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
        Ok(self.right)
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
    let mut feed = ControlledFeed {
        left,
        right,
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
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    JoinItem.stream(&operation, &mut feed, &mut context, room, &mut emitted)?;
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
        let (rows, holds) = stream_lookup(&mut room, &left, right, false, false, false)?;
        if !rows.is_empty() || holds != 1 || room.scratch.right != Table::default() {
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
    let (rows, holds) = stream_lookup(&mut room, &left, right, false, false, false)?;
    let expected = [vec![number(20)], vec![number(21)], vec![number(22)]];
    Ok(Value::from(rows == expected && holds == 1))
}

fn a_reused_room_skips_then_replaces_its_right_table() -> Result<Value, OperatingError> {
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
    let retained = room.scratch.right.clone();
    let skipped = stream_lookup(&mut room, &empty_left, middle_right, false, false, false)?.0;
    let unchanged = room.scratch.right == retained;
    let last = stream_lookup(&mut room, &last_left, last_right, false, false, false)?.0;
    Ok(Value::from(
        first == [vec![number(11)], vec![number(12)]]
            && skipped.is_empty()
            && unchanged
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
    let result = stream_lookup(&mut room, &left, right, true, false, false);
    if room.scratch.right.count == RowCount(0) && room.scratch.right.cells.is_empty() {
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
    let result = stream_lookup(&mut room, &left, right, false, false, true);
    if room.scratch.right.count == RowCount(1) {
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
        input: || a_reused_room_skips_then_replaces_its_right_table,
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
    super::super::fixtures::contract::check(QueryOperator::Join);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
