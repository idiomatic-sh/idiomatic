//! Independent tree and scalar Join expectations through the compiled owner.
use super::fixtures::{
    edges_to_a_few_named_rows, edges_to_a_second_child, edges_to_children_of_kind_inner,
    elements_covering_one_to_two, elements_whose_text_is_b, joins_the_tree,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{Integer, NodeId, Offset, Span, Value as Cell};
use std::borrow::Cow;

/// A join of a wide tree's nodes, edges or elements, compiled through the owner.
#[derive(Clone, Copy, Debug)]
enum Join {
    /// Nodes at `starts` joined with the tree's edges on edge column `on`: 1 is the child, 0
    /// the parent.
    Tree { starts: &'static [u32], on: u32 },
    /// Nodes 1, 3 and 2 joined with the elements on their ids, keeping text `b`.
    ElementsWhoseTextIsB,
    /// The root joined with the edges whose child is its second.
    EdgesToASecondChild,
    /// Every edge of a 40-token tree joined, by its child, with the elements of kind `inner`.
    EdgesToChildrenOfKindInner,
    /// Every edge of a 40-token tree joined, by its child, with three rows naming nodes.
    EdgesToAFewNamedRows,
    /// The stretch 1..2 joined with the elements of a 40-token tree on their stretch alone.
    ElementsCoveringOneToTwo,
}

/// The node at `at` of file 0.
const fn node(at: u64) -> Cell {
    Cell::Node(NodeId(at))
}

const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

const fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

type Input = Join;
/// The joined cells, row after row.
type Value = Cow<'static, [Cell]>;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exact_tree_join_rows_filters_views_and_order"),
        word: Word::Succeeds,
        input: || Join::Tree {
            starts: &[3, 1],
            on: 1,
        },
        // A node joined on the child of the tree finds its parent.
        expect: Expect::Value(Cow::Borrowed(&[node(2), node(3), node(0), node(1)])),
    },
    Case {
        name: CaseName("a_node_joined_on_the_parent_of_the_tree_finds_its_children"),
        word: Word::Succeeds,
        input: || Join::Tree {
            starts: &[0],
            on: 0,
        },
        expect: Expect::Value(Cow::Borrowed(&[node(0), node(1), node(0), node(2)])),
    },
    Case {
        name: CaseName("the_root_has_no_parent"),
        word: Word::Succeeds,
        input: || Join::Tree {
            starts: &[0],
            on: 1,
        },
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("a_leaf_has_no_children"),
        word: Word::Succeeds,
        input: || Join::Tree {
            starts: &[3],
            on: 0,
        },
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("only_the_elements_a_filter_admits_are_joined_by_their_ids"),
        word: Word::Succeeds,
        input: || Join::ElementsWhoseTextIsB,
        expect: Expect::Value(Cow::Borrowed(&[node(3), stretch(1, 2)])),
    },
    Case {
        name: CaseName("only_the_edges_a_filter_admits_are_joined"),
        word: Word::Succeeds,
        input: || Join::EdgesToASecondChild,
        expect: Expect::Value(Cow::Borrowed(&[node(2)])),
    },
    Case {
        name: CaseName("the_edges_of_the_children_of_a_kind_are_found_by_the_kind"),
        word: Word::Succeeds,
        input: || Join::EdgesToChildrenOfKindInner,
        expect: Expect::Value(Cow::Borrowed(&[node(0), node(41)])),
    },
    Case {
        name: CaseName("a_few_rows_leave_in_the_order_of_the_edges"),
        word: Word::Succeeds,
        input: || Join::EdgesToAFewNamedRows,
        expect: Expect::Value(Cow::Borrowed(&[
            node(5),
            number(1),
            node(5),
            number(3),
            node(41),
            number(2),
        ])),
    },
    Case {
        name: CaseName("the_elements_covering_a_stretch_are_found_by_it_alone"),
        word: Word::Succeeds,
        input: || Join::ElementsCoveringOneToTwo,
        expect: Expect::Value(Cow::Borrowed(&[node(41), node(42)])),
    },
];

/// Compiles and runs the join, answering its cells.
fn subject(input: Input) -> Result<Value, Failure> {
    let joined = match input {
        Join::Tree { starts, on } => joins_the_tree(starts, on),
        Join::ElementsWhoseTextIsB => elements_whose_text_is_b(),
        Join::EdgesToASecondChild => edges_to_a_second_child(),
        Join::EdgesToChildrenOfKindInner => edges_to_children_of_kind_inner(),
        Join::EdgesToAFewNamedRows => edges_to_a_few_named_rows(),
        Join::ElementsCoveringOneToTwo => elements_covering_one_to_two(),
    }?;
    Ok(Cow::Owned(joined.cells))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
