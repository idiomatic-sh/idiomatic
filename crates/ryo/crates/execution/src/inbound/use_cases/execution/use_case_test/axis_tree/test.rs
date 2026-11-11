//! Axis walks over the syntax tree through the mandatory public execution owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::service::node_position;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// The positions of the nodes found, in order.
type Value = Cow<'static, [u32]>;
type Failure = OperatingError;
/// A direction, and the position of the node it walks from.
type Input = (Direction, u32);
const FROM: Source = Source::Relation(Relation::Function);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}
fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}
/// What the third input of a walk over the tree is: the nodes of a table, or the elements of the tree.
pub(crate) enum Domain {
    /// A table of nodes.
    Nodes(Table),
    /// The elements of the tree.
    Elements,
}
/// A table of one node column holding `starts`.
fn node_table(starts: impl IntoIterator<Item = u32>) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Node],
        ..Table::default()
    };
    starts
        .into_iter()
        .for_each(|position| push_row(&mut table, &[node(position)]));
    table
}
/// The axis operation that walks `direction` over the tree, from column 0 to column 1 of the edges, with the domain's filter and output.
fn axis(direction: Direction, filter: Vec<PlanComparison>, output: Vec<JoinColumn>) -> Operation {
    Operation::Axis {
        from: FROM,
        edges: FROM,
        domain: FROM,
        direction,
        id: column(0),
        parent: column(0),
        child: column(1),
        domain_id: column(0),
        domain_filter: filter,
        domain_view: Vec::new(),
        output,
    }
}
/// What the item makes of the operation over the tree, from the nodes in `starts`.
pub(crate) fn over_the_tree(
    operation: &Operation,
    starts: &Table,
    domain: &Domain,
) -> Result<Table, OperatingError> {
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{Flag, Span},
    };
    let (elements, mut symbols) = fixtures::requests::tree_with_names();
    let word = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b"abcd"),
        ryo_relations::domain::terms::types::NameText("fixture"),
    );
    let facts = |relation, table: &Table| {
        let mut supplied = Table {
            kinds: kinds(relation).to_vec(),
            ..Table::default()
        };
        for row in table.cells.chunks(table.kinds.len()) {
            supplied
                .cells
                .extend(kinds(relation).iter().map(|kind| match kind {
                    CellKind::Node => row[0],
                    CellKind::Symbol => Cell::Symbol(word),
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Cell::Number(Integer(0)),
                    CellKind::Flag => Cell::Flag(Flag::Lowered),
                }));
            supplied.count.0 += 1;
        }
        supplied
    };
    let from = facts(Relation::Function, starts);
    let domain_rows = match domain {
        Domain::Nodes(table) => Some(facts(Relation::Identifier, table)),
        Domain::Elements => None,
    };
    let mut operations = vec![Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![column(0)],
    }];
    let domain_source = if domain_rows.is_some() {
        operations.push(Operation::Select {
            from: Source::Relation(Relation::Identifier),
            comparisons: vec![],
            kept: vec![column(0)],
        });
        Source::Step(ryo_values::domain::plan_parts::types::StepId(1))
    } else {
        Source::Relation(Relation::SyntaxTree)
    };
    let mut operation = operation.clone();
    let Operation::Axis {
        from: from_input,
        edges,
        domain,
        ..
    } = &mut operation
    else {
        panic!("the fixture builds an Axis")
    };
    *from_input = Source::Step(ryo_values::domain::plan_parts::types::StepId(0));
    *edges = Source::Relation(Relation::SyntaxChild);
    *domain = domain_source;
    operations.push(operation);
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the fake serves the axis's source and its declared domain; every other relation is not read"
    )]
    let read = |relation| {
        match relation {
        Relation::Function => Ok(ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows::Computed(&from)),
        Relation::Identifier => Ok(ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows::Computed(
            domain_rows.as_ref().expect("declared domain"),
        )),
        _ => Err(OperatingError::NotRead),
    }
    };
    fixtures::requests::published_from(
        inputs::plan(operations),
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
        &read,
    )
}
/// The entities found walking `direction` from the node at `start` in the tree, among all the nodes of the file.
fn walks_the_tree(direction: Direction, start: u32) -> Result<Vec<Cell>, OperatingError> {
    let out = over_the_tree(
        &axis(direction, Vec::new(), Vec::new()),
        &node_table([start]),
        &Domain::Nodes(node_table(0..4)),
    )?;
    let mut found: Vec<Cell> = out.cells.chunks(2).map(|row| row[1]).collect();
    found.sort();
    Ok(found)
}
/// The rows of this part's cases: a direction and the node it walks from, and the positions it finds.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("descendants_in_the_tree_are_everything_below"),
        word: Word::Succeeds,
        input: || (Direction::Descendant, 0),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3])),
    },
    Case {
        name: CaseName("the_descendants_of_the_node_at_2_are_the_token_below_it"),
        word: Word::Succeeds,
        input: || (Direction::Descendant, 2),
        expect: Expect::Value(Cow::Borrowed(&[3])),
    },
    Case {
        name: CaseName("the_token_at_3_has_no_descendants"),
        word: Word::Succeeds,
        input: || (Direction::Descendant, 3),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("ancestors_in_the_tree_are_the_parents_up_to_the_root"),
        word: Word::Succeeds,
        input: || (Direction::Ancestor, 3),
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
    Case {
        name: CaseName("the_root_has_no_ancestors"),
        word: Word::Succeeds,
        input: || (Direction::Ancestor, 0),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("the_nearest_ancestor_in_the_tree_is_the_parent"),
        word: Word::Succeeds,
        input: || (Direction::NearestAncestor, 3),
        expect: Expect::Value(Cow::Borrowed(&[2])),
    },
    Case {
        name: CaseName("the_parent_and_the_children_in_the_tree"),
        word: Word::Succeeds,
        input: || (Direction::Parent, 3),
        expect: Expect::Value(Cow::Borrowed(&[2])),
    },
    Case {
        name: CaseName("the_children_of_the_root_are_the_nodes_at_1_and_2"),
        word: Word::Succeeds,
        input: || (Direction::Child, 0),
        expect: Expect::Value(Cow::Borrowed(&[1, 2])),
    },
    Case {
        name: CaseName("the_root_has_no_parent"),
        word: Word::Succeeds,
        input: || (Direction::Parent, 0),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent"),
        word: Word::Succeeds,
        input: || (Direction::PreviousSibling, 2),
        expect: Expect::Value(Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("the_next_sibling_of_the_first_child_is_the_second"),
        word: Word::Succeeds,
        input: || (Direction::NextSibling, 1),
        expect: Expect::Value(Cow::Borrowed(&[2])),
    },
    Case {
        name: CaseName("the_first_child_has_no_previous_sibling"),
        word: Word::Succeeds,
        input: || (Direction::PreviousSibling, 1),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("the_last_child_has_no_next_sibling"),
        word: Word::Succeeds,
        input: || (Direction::NextSibling, 2),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("an_only_child_has_no_next_sibling"),
        word: Word::Succeeds,
        input: || (Direction::NextSibling, 3),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("the_root_has_no_previous_sibling"),
        word: Word::Succeeds,
        input: || (Direction::PreviousSibling, 0),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
];
fn subject((direction, start): Input) -> Result<Value, Failure> {
    walks_the_tree(direction, start).map(|found| {
        found
            .iter()
            .filter_map(|cell| {
                if let Cell::Node(node) = cell {
                    Some(node_position(*node).0)
                } else {
                    None
                }
            })
            .collect()
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
