//! The `tree_join` service's cases, on values alone.

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    FileId, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_relations::domain::terms::types::NameText;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::join::item::JoinItem;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;

use crate::domain::tree_join::service::{source_column, tree_finding, tree_value};
use ryo_derived::domain::contract::types::Finding;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::{
    Comparison, Input as PlanInput, JoinColumn, LiteralText, Operand, QueryLiteral, Side, Test,
};
use ryo_values::domain::plans::types::Operation;

/// What a row asks: which part of the service it checks.
#[derive(Clone, Copy)]
enum Scenario {
    /// The cells of an element are read by column.
    ElementCells,
    /// The cells of an edge are read by column.
    EdgeCells,
    /// A column of a view is the column of the elements it names.
    ViewedColumn,
    /// A key that is the stretch of an element is found by the stretch.
    FindingBySpan,
    /// A key that is a node is found by the node.
    FindingByNode,
    /// A file and a stretch, in either order, are found by the stretch of the file.
    FindingByStretch,
    /// A key that no strategy finds is found by no finding.
    FindingByNothing,
    /// A tree on either side of a join gives the same rows: on the right read through a view and kept to the tokens, on the left whole.
    EitherSide,
}

/// What a row asks.
type Input = Scenario;
/// Every assertion of the scenario held.
type Value = ();
type Failure = ();

/// The tree of a file: a root holding a token and a node, which holds a token. Positions 0 to 3, the text `abcd`.
fn tree() -> Elements {
    tree_with_names().0
}

/// The same tree with the names its texts were made with.
fn tree_with_names() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(3), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    // A language names every kind its table holds.
    for (bits, name) in [(1, "file"), (2, "word"), (3, "group")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(bits),
            NameText(name),
        );
    }
    (elements, symbols)
}

fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn found(
    form: TreeForm,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
) -> Option<Finding> {
    let elements = tree();
    let rows = TreeRows {
        elements: &elements,
        file: FileId(0),
        form,
    };
    tree_finding(&rows, left_on, right_on, view)
}

/// A join reads the elements of the tree on either side of it.
fn either_side() {
    let tokens = {
        let mut table = Table {
            kinds: vec![CellKind::Node],
            ..Table::default()
        };
        push_row(&mut table, &[node(1)]);
        push_row(&mut table, &[node(3)]);
        table
    };
    let has_text = Comparison {
        test: Test::NotEqual,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::default()))),
        origin: None,
    };
    let join = |filter: Vec<Comparison>, view: Vec<ColumnIndex>, output: Vec<JoinColumn>| {
        Operation::Join {
            left: PlanInput::Relation(ryo_relations::domain::relations::types::Relation::Function),
            right: PlanInput::Relation(ryo_relations::domain::relations::types::Relation::Function),
            left_on: vec![column(0)],
            right_on: vec![column(0)],
            output,
            filter,
            view,

            reach: None,
        }
    };
    let output = |side: Side, place: u32| JoinColumn {
        side,
        column: column(place),
    };
    let (elements, mut symbols) = tree_with_names();
    let mut made = |operation: &Operation, inputs: &[Rows<'_>]| {
        let mut out = Table::default();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(b"abcd"),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut OperatorScratch::default(),
        };
        JoinItem
            .compute(operation, inputs, &mut context, &mut out)
            .expect("the join is computed");
        out
    };
    let rows = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let on_the_right = made(
        &join(
            vec![has_text],
            vec![column(0), column(2)],
            vec![output(Side::Left, 0), output(Side::Right, 1)],
        ),
        &[Rows::Computed(&tokens), Rows::Tree(rows)],
    );
    let on_the_left = made(
        &join(
            Vec::new(),
            Vec::new(),
            vec![output(Side::Left, 0), output(Side::Left, 2)],
        ),
        &[Rows::Tree(rows), Rows::Computed(&tokens)],
    );
    assert_eq!(on_the_right.count.0, 2);
    assert_eq!(on_the_left.count.0, 2);
    assert_eq!(on_the_right.cells, on_the_left.cells);
}

/// The cells of the element at `at` of `elements`, read by column from the rows of the elements.
fn element_cells(elements: &Elements, at: Position) {
    let rows = TreeRows {
        elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    assert_eq!(tree_value(&rows, at, column(0)), node(2));
    assert_eq!(tree_value(&rows, at, column(3)), Cell::File(FileId(0)));
    assert_eq!(
        tree_value(&rows, at, column(4)),
        Cell::Span(Span {
            start: Offset(1),
            end: Offset(2),
        })
    );
    assert_eq!(tree_value(&rows, at, column(9)), Cell::Flag(Flag::Lowered));
}

/// The cells of the edge to the element at `at` of `elements`, read by column from the rows of the edges.
fn edge_cells(elements: &Elements, at: Position) {
    let rows = TreeRows {
        elements,
        file: FileId(0),
        form: TreeForm::Edges,
    };
    assert_eq!(tree_value(&rows, at, column(0)), node(0));
    assert_eq!(tree_value(&rows, at, column(1)), node(2));
    assert_eq!(tree_value(&rows, at, column(2)), Cell::Number(Integer(1)));
    assert_eq!(tree_value(&rows, at, column(3)), Cell::Symbol(NO_SYMBOL));
    assert_eq!(tree_value(&rows, at, column(4)), Cell::File(FileId(0)));
    assert_eq!(tree_value(&rows, at, column(9)), Cell::Flag(Flag::Lowered));
}

fn subject(scenario: Input) {
    let elements = tree();
    let at = Position(2);
    match scenario {
        Scenario::ElementCells => element_cells(&elements, at),
        Scenario::EdgeCells => edge_cells(&elements, at),
        Scenario::ViewedColumn => {
            let view = [column(1), column(4)];
            assert_eq!(source_column(&view, column(1)), column(4));
            assert_eq!(source_column(&view, column(5)), column(5));
            assert_eq!(source_column(&[], column(2)), column(2));
        }
        Scenario::FindingBySpan => {
            let by_span = found(TreeForm::Elements, &[column(0)], &[column(4)], &[]);
            assert!(matches!(by_span, Some(Finding::Span(ColumnIndex(0)))));
            let viewed = found(
                TreeForm::Elements,
                &[column(0)],
                &[column(1)],
                &[column(0), column(4)],
            );
            assert!(matches!(viewed, Some(Finding::Span(ColumnIndex(0)))));
        }
        Scenario::FindingByNode => {
            let by_id = found(TreeForm::Elements, &[column(0)], &[column(0)], &[]);
            assert!(matches!(
                by_id,
                Some(Finding::Node(ColumnIndex(0), ColumnIndex(0)))
            ));
            let by_child = found(TreeForm::Edges, &[column(0)], &[column(1)], &[]);
            assert!(matches!(
                by_child,
                Some(Finding::Node(ColumnIndex(0), ColumnIndex(1)))
            ));
        }
        Scenario::FindingByStretch => {
            let file_first = found(
                TreeForm::Elements,
                &[column(0), column(1)],
                &[column(3), column(4)],
                &[],
            );
            assert!(matches!(
                file_first,
                Some(Finding::Stretch(ColumnIndex(0), ColumnIndex(1)))
            ));
            let stretch_first = found(
                TreeForm::Elements,
                &[column(0), column(1)],
                &[column(4), column(3)],
                &[],
            );
            assert!(matches!(
                stretch_first,
                Some(Finding::Stretch(ColumnIndex(1), ColumnIndex(0)))
            ));
        }
        Scenario::EitherSide => either_side(),
        Scenario::FindingByNothing => {
            assert!(found(TreeForm::Elements, &[column(0)], &[column(1)], &[]).is_none());
            assert!(
                found(
                    TreeForm::Edges,
                    &[column(0), column(1)],
                    &[column(3), column(4)],
                    &[]
                )
                .is_none()
            );
            assert!(found(TreeForm::Elements, &[], &[], &[]).is_none());
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_cells_of_an_element_are_read_by_column"),
        word: Word::Succeeds,
        input: || Scenario::ElementCells,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_cells_of_an_edge_are_read_by_column"),
        word: Word::Succeeds,
        input: || Scenario::EdgeCells,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_column_of_a_view_is_the_column_it_names"),
        word: Word::Succeeds,
        input: || Scenario::ViewedColumn,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_stretch_key_is_found_by_the_stretch"),
        word: Word::Succeeds,
        input: || Scenario::FindingBySpan,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_node_key_is_found_by_the_node"),
        word: Word::Succeeds,
        input: || Scenario::FindingByNode,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_file_and_a_stretch_are_found_by_the_stretch_of_the_file"),
        word: Word::Succeeds,
        input: || Scenario::FindingByStretch,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_tree_joins_as_many_rows_on_either_side"),
        word: Word::Succeeds,
        input: || Scenario::EitherSide,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_key_no_strategy_finds_is_found_by_no_finding"),
        word: Word::Succeeds,
        input: || Scenario::FindingByNothing,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        subject(input);
        Ok(())
    });
}

use ryo_values::domain::rows::types::Table;
