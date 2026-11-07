//! Axis walks over the syntax tree through the mandatory public execution owner.
use super::super::super::fixtures;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use crate::domain::operating::errors::OperatingError;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Position;
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

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;
const FROM: Source = Source::Relation(Relation::Function);
fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}
fn node(position: u32) -> Cell {
    Cell::Node(node_id(FileId(0), Position(position)))
}
/// What the third input of a walk over the tree is: the nodes of a table, or the elements of the tree.
enum Domain {
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
fn over_the_tree(
    operation: &Operation,
    starts: &Table,
    domain: &Domain,
) -> Result<Table, OperatingError> {
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{Flag, Span},
    };
    let (elements, mut symbols) = fixtures::tree_with_names();
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
        unreachable!()
    };
    *from_input = Source::Step(ryo_values::domain::plan_parts::types::StepId(0));
    *edges = Source::Relation(Relation::SyntaxChild);
    *domain = domain_source;
    operations.push(operation);
    let read = |relation| match relation {
        Relation::Function => Ok(super::super::super::types::SourceRows::Computed(&from)),
        Relation::Identifier => Ok(super::super::super::types::SourceRows::Computed(
            domain_rows.as_ref().expect("declared domain"),
        )),
        _ => Err(OperatingError::NotRead),
    };
    fixtures::published_from(
        fixtures::plan(operations),
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
fn descendants_in_the_tree_are_everything_below() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Descendant, 0)? == [node(1), node(2), node(3)]
            && walks_the_tree(Direction::Descendant, 2)? == [node(3)]
            && walks_the_tree(Direction::Descendant, 3)?.is_empty(),
    ))
}
fn ancestors_in_the_tree_are_the_parents_up_to_the_root() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Ancestor, 3)? == [node(0), node(2)]
            && walks_the_tree(Direction::Ancestor, 0)?.is_empty(),
    ))
}
fn the_nearest_ancestor_in_the_tree_is_the_parent() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::NearestAncestor, 3)? == [node(2)],
    ))
}
fn the_parent_and_the_children_in_the_tree() -> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::Parent, 3)? == [node(2)]
            && walks_the_tree(Direction::Child, 0)? == [node(1), node(2)]
            && walks_the_tree(Direction::Parent, 0)?.is_empty(),
    ))
}
fn the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent()
-> Result<Value, OperatingError> {
    Ok(usize::from(
        walks_the_tree(Direction::PreviousSibling, 2)? == [node(1)]
            && walks_the_tree(Direction::NextSibling, 1)? == [node(2)]
            && walks_the_tree(Direction::PreviousSibling, 1)?.is_empty()
            && walks_the_tree(Direction::NextSibling, 2)?.is_empty()
            && walks_the_tree(Direction::NextSibling, 3)?.is_empty()
            && walks_the_tree(Direction::PreviousSibling, 0)?.is_empty(),
    ))
}
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("descendants_in_the_tree_are_everything_below"),
        word: Word::Succeeds,
        input: || descendants_in_the_tree_are_everything_below,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("ancestors_in_the_tree_are_the_parents_up_to_the_root"),
        word: Word::Succeeds,
        input: || ancestors_in_the_tree_are_the_parents_up_to_the_root,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_nearest_ancestor_in_the_tree_is_the_parent"),
        word: Word::Succeeds,
        input: || the_nearest_ancestor_in_the_tree_is_the_parent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_parent_and_the_children_in_the_tree"),
        word: Word::Succeeds,
        input: || the_parent_and_the_children_in_the_tree,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent"),
        word: Word::Succeeds,
        input: || the_siblings_in_the_tree_are_the_children_beside_under_the_same_parent,
        expect: Expect::Value(1),
    },];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
