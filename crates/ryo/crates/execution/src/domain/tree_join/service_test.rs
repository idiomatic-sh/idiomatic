//! Native cell reading and lookup strategy decisions use their actual pure owners.
use crate::domain::{
    table_cells::types::{TreeForm, TreeRows},
    tree_join::service::{driver::tree_value, source_column},
};
use ryo_derived::{
    domain::contract::types::{
        BuildOutcome, BuildWorkspace, DerivedInputs, DerivedView, Finding, NativeForm, ViewRequest,
    },
    inbound::use_cases::derived::interface::DerivedUseCase,
};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::{
    completion::service::{completion, name_kind},
    elements::types::Elements,
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::{
        constants::NO_SYMBOL,
        service::node_id,
        types::{CellKind, FileId, Flag, Integer, KindBits, Offset, Position, Span, Value as Cell},
    },
    terms::types::NameText,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::ColumnIndex,
};
#[derive(Clone, Copy)]
enum Scenario {
    ElementCells,
    EdgeCells,
    ViewedColumn,
    FindingBySpan,
    FindingByNode,
    FindingByStretch,
    FindingByNothing,
}
type Input = Scenario;
type Value = ();
type Failure = ();
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
    let semantics = ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new();
    let native = ryo_relations::domain::relations::service::kinds(match form {
        TreeForm::Elements => ryo_relations::domain::relations::types::Relation::SyntaxTree,
        TreeForm::Edges => ryo_relations::domain::relations::types::Relation::SyntaxChild,
    });
    let mut left = vec![
        CellKind::Node;
        left_on
            .iter()
            .map(|column| column.0 as usize + 1)
            .max()
            .unwrap_or(0)
    ];
    for (left_column, right_column) in left_on.iter().zip(right_on) {
        left[left_column.0 as usize] = native[source_column(view, *right_column).0 as usize];
    }
    let prepared = semantics.prepare(
        ViewRequest::NativeLookup {
            left_on: left_on.to_vec(),
            right_on: right_on.to_vec(),
            view: view.to_vec(),
        },
        vec![left, native.to_vec()],
    );
    let form = match form {
        TreeForm::Elements => NativeForm::Elements,
        TreeForm::Edges => NativeForm::Edges,
    };
    let BuildOutcome::Built(payload) = semantics
        .build(
            &prepared,
            DerivedInputs::Native(form),
            &mut BuildWorkspace::default(),
            None,
        )
        .expect("finite native lookup")
    else {
        return None;
    };
    let DerivedView::NativeLookup(finding) =
        semantics.view(&prepared, &payload).expect("lookup view")
    else {
        panic!("native lookup family")
    };
    Some(*finding)
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
