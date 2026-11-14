//! The `directed_walk` item of the `kernels` collection: from each start row, in order, the entities a walk over the edges
//! reaches in its direction that the domain admits, in walk order; the nearest ancestor the first only. Over the tree's
//! edges (`syntax-child`) the walk reads the open file's elements in place, as `scan_roots` does; over any other edges, the
//! edges are keyed by parent or child through `hash_index`'s table, and the domain's entities through its second instance.

use super::super::filter::item::{filter, keep};
use super::super::hash_index::item::{cells, find_value, keyed_walk};
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::axis::types::AxisParts;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::{
    KeyedRows, NodeDecisions, WalkScratch,
};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{InputRows, KernelRun, KeyedSide, TreeRead};
use crate::domain::table_scratch::types::SetMembership;
use crate::domain::tree_cells::service::{tree_value, walked_tree};
use crate::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{CellKind, FileId, Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `directed_walk`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct DirectedWalkItem;

#[cfg(test)]
impl KernelsItem for DirectedWalkItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Axis { .. } => true,
            Lowering::Filter { .. }
            | Lowering::Lookup { .. }
            | Lowering::Project { .. }
            | Lowering::Extend { .. }
            | Lowering::Probe { .. }
            | Lowering::Distinct { .. }
            | Lowering::Group { .. }
            | Lowering::Reach { .. }
            | Lowering::SeededReach { .. }
            | Lowering::OrderedWalk { .. }
            | Lowering::Components { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The three inputs of an axis: its start rows, its edge rows and its domain rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AxisInputs<'a> {
    /// The start rows.
    pub(crate) starts: InputRows<'a>,
    /// The edge rows.
    pub(crate) edges: InputRows<'a>,
    /// The domain rows.
    pub(crate) domain: InputRows<'a>,
}

/// What an axis borrows besides its rows: the two keyed-row tables (its edges and its domain's entities) and its domain
/// filter's decisions.
#[derive(Debug)]
pub(crate) struct AxisTables<'s> {
    /// The edges, keyed by parent or child.
    pub(crate) edges: &'s mut KeyedRows,
    /// The domain's entities.
    pub(crate) domain: &'s mut KeyedRows,
    /// The domain filter's decisions.
    pub(crate) decisions: &'s mut NodeDecisions,
}

/// How a walk reads its edges and admits what it reaches.
#[derive(Clone, Copy)]
enum Mode<'a> {
    /// A domain of no rows and no declared output: no start reaches anything.
    Empty,
    /// The tree's own edges, walked over the elements; the domain admits an element by its filter over the domain's tree
    /// rows (`among`), or by holding its node.
    Native {
        tree: &'a TreeRead<'a>,
        among: Option<&'a TreeRead<'a>>,
    },
    /// Edges keyed by `from`, leading to `to`; the domain admits the entities it holds.
    Indexed { from: ColumnIndex, to: ColumnIndex },
}

/// Writes into `output` each start row's answers, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows, or an indexed walk with a domain filter, a domain view, a
/// declared output or a direction only a tree has.
pub(crate) fn walked(
    axis: &AxisParts,
    inputs: AxisInputs<'_>,
    run: &KernelRun<'_>,
    tables: AxisTables<'_>,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let AxisInputs {
        starts,
        edges,
        domain,
    } = inputs;
    let starts_count = count_of(starts).0;
    output.kinds.clear();
    if starts_count == 0 {
        // No start acquires the walk's resources: an axis over no start rows answers none and raises nothing.
        output.kinds.extend_from_slice(kinds_of(starts));
        return Ok(None);
    }
    let mode = mode_of(axis, edges, domain)?;
    let reached_kinds = match mode {
        Mode::Native { tree, among } => among.unwrap_or(tree).kinds,
        Mode::Empty | Mode::Indexed { .. } => kinds_of(edges),
    };
    let elements = matches!(mode, Mode::Native { .. });
    if !elements || axis.output.is_empty() {
        output.kinds.extend_from_slice(kinds_of(starts));
        output.kinds.push(match mode {
            Mode::Indexed { to, .. } => kind_at(kinds_of(edges), to)?,
            Mode::Empty | Mode::Native { .. } => CellKind::Node,
        });
    } else {
        for column in &axis.output {
            output.kinds.push(match column.side {
                Side::Left => kind_at(kinds_of(starts), column.column)?,
                Side::Right => kind_at(reached_kinds, column.column)?,
            });
        }
    }
    let AxisTables {
        edges: index,
        domain: members,
        decisions,
    } = tables;
    let domain_side = KeyedSide {
        rows: domain,
        on: std::slice::from_ref(&axis.domain_id),
    };
    let marks = match mode {
        Mode::Native { among: None, .. } | Mode::Indexed { .. } => {
            cells(domain_side, members)?;
            true
        }
        Mode::Native { among: Some(_), .. } | Mode::Empty => false,
    };
    if let Mode::Indexed { from, .. } = mode {
        cells(
            KeyedSide {
                rows: edges,
                on: std::slice::from_ref(&from),
            },
            index,
        )?;
    }
    let among_kinds = match mode {
        Mode::Native {
            among: Some(among), ..
        } => among.kinds,
        Mode::Empty | Mode::Native { among: None, .. } | Mode::Indexed { .. } => &[],
    };
    let verdicts = filter(&axis.domain, among_kinds, run.resources.texts(), decisions);
    for at in 0..starts_count {
        let row = RowIndex(at);
        let start = cell_of(starts, row, axis.id)
            .ok_or(PortableOperatorFault::NoSuchColumn { index: axis.id })?;
        walk.reached.clear();
        match mode {
            Mode::Empty => {}
            Mode::Native { tree, .. } => native(tree, axis.direction, start, &mut walk.reached),
            Mode::Indexed { from, to } => {
                let edge_side = KeyedSide {
                    rows: edges,
                    on: std::slice::from_ref(&from),
                };
                let admitted = |entity: Value| -> Result<SetMembership, PortableOperatorFault> {
                    Ok(match find_value(members, domain_side, entity)? {
                        Some(_) => SetMembership::In,
                        None => SetMembership::Out,
                    })
                };
                keyed_walk(edge_side, index, to, axis.direction, start, walk, admitted)?;
            }
        }
        for place in 0..walk.reached.len() {
            let candidate = walk.reached[place];
            let admitted = match mode {
                Mode::Empty => false,
                Mode::Native {
                    among: Some(among), ..
                } => {
                    let position = node_position_of(candidate);
                    let reached = tree_rows(among, candidate);
                    walk.entities.clear();
                    walk.entities.extend(
                        (0..)
                            .map(ColumnIndex)
                            .take(among.kinds.len())
                            .map(|column| tree_value(&reached, position, column)),
                    );
                    walk.rows.cells.clear();
                    walk.rows.count = RowCount(0);
                    if let Err(fault) = keep(
                        verdicts,
                        run.matcher,
                        &walk.entities,
                        decisions,
                        &mut walk.written,
                        &mut walk.rows,
                    ) {
                        return Ok(Some(fault));
                    }
                    walk.rows.count.0 > 0
                }
                Mode::Native { among: None, .. } | Mode::Indexed { .. } => {
                    marks && find_value(members, domain_side, candidate)?.is_some()
                }
            };
            if !admitted {
                continue;
            }
            if !elements || axis.output.is_empty() {
                for column in (0..).map(ColumnIndex).take(kinds_of(starts).len()) {
                    output.cells.push(
                        cell_of(starts, row, column)
                            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
                    );
                }
                output.cells.push(candidate);
            } else {
                let Mode::Native { tree, among } = mode else {
                    return Err(PortableOperatorFault::ShapeMismatch);
                };
                let reached = tree_rows(among.unwrap_or(tree), candidate);
                for column in &axis.output {
                    output.cells.push(match column.side {
                        Side::Left => cell_of(starts, row, column.column).ok_or(
                            PortableOperatorFault::NoSuchColumn {
                                index: column.column,
                            },
                        )?,
                        Side::Right => {
                            tree_value(&reached, node_position_of(candidate), column.column)
                        }
                    });
                }
            }
            output.count = RowCount(output.count.0 + 1);
            if axis.direction == Direction::NearestAncestor {
                break;
            }
        }
    }
    Ok(None)
}

/// How the axis reads its edges and admits what it reaches, decided as main's engine decides it.
///
/// # Errors
///
/// `WrongOperation` for an indexed walk with a domain filter, a domain view, a declared output or a direction only a tree
/// has; `NoSuchColumn` for an edge or domain column past its rows.
fn mode_of<'a>(
    axis: &AxisParts,
    edges: InputRows<'a>,
    domain: InputRows<'a>,
) -> Result<Mode<'a>, PortableOperatorFault> {
    let filtered = !axis.domain.comparisons.is_empty();
    let view = &axis.domain.kept;
    if matches!(domain, InputRows::Table(_)) && count_of(domain).0 == 0 && axis.output.is_empty() {
        return Ok(Mode::Empty);
    }
    if let InputRows::Tree(tree) = edges {
        let among = match domain {
            InputRows::Tree(each) if each.form == TreeForm::Elements => Some(each),
            InputRows::Tree(_) | InputRows::Facts(_) | InputRows::Table(_) => None,
        };
        let entity = view
            .get(usize::from(axis.domain_id))
            .copied()
            .unwrap_or(axis.domain_id);
        if tree.form == TreeForm::Edges
            && (axis.parent, axis.child) == (ColumnIndex(0), ColumnIndex(1))
            && ((!filtered && view.is_empty()) || (among.is_some() && entity == ColumnIndex(0)))
        {
            return Ok(Mode::Native { tree, among });
        }
    }
    if filtered
        || !view.is_empty()
        || !axis.output.is_empty()
        || matches!(
            axis.direction,
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
        )
    {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let _parent = kind_at(kinds_of(edges), axis.parent)?;
    let _child = kind_at(kinds_of(edges), axis.child)?;
    let _entity = kind_at(kinds_of(domain), axis.domain_id)?;
    let (from, to) = match axis.direction {
        Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => {
            (axis.child, axis.parent)
        }
        Direction::Child
        | Direction::Descendant
        | Direction::Deep
        | Direction::PreviousSibling
        | Direction::NextSibling => (axis.parent, axis.child),
    };
    Ok(Mode::Indexed { from, to })
}

/// The kind of `column` among `kinds`.
fn kind_at(kinds: &[CellKind], column: ColumnIndex) -> Result<CellKind, PortableOperatorFault> {
    kinds
        .get(usize::from(column))
        .copied()
        .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// The position of a candidate's node; past every element for a candidate that is no node.
fn node_position_of(candidate: Value) -> Position {
    if let Value::Node(node) = candidate {
        node_position(node)
    } else {
        Position(u32::MAX)
    }
}

/// The tree rows of `tree`'s form in the file of `candidate`'s node.
fn tree_rows<'a>(tree: &TreeRead<'a>, candidate: Value) -> TreeRows<'a> {
    let file = if let Value::Node(node) = candidate {
        node_file(node)
    } else {
        FileId(u32::MAX)
    };
    TreeRows {
        form: tree.form,
        elements: tree.elements,
        file,
    }
}

/// Appends to `found` the nodes the tree's walk from `start` reaches in `direction`, in walk order; a start that is no node
/// of the open file reaches none.
fn native(tree: &TreeRead<'_>, direction: Direction, start: Value, found: &mut Vec<Value>) {
    let Value::Node(start) = start else {
        return;
    };
    let elements = tree.elements;
    let file = node_file(start);
    let at = node_position(start);
    let in_file = tree
        .candidates
        .first()
        .is_none_or(|first| node_file(*first) == file);
    if !in_file || !usize::try_from(at.0).is_ok_and(|place| place < elements.kind.len()) {
        return;
    }
    walked_tree(elements, file, direction, at, found);
}
