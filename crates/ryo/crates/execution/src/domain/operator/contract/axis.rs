//! Borrowed axis resources exposed by the engine; primitives cannot retain cache handles. The engine's walk
//! over them (a tree of elements, or an index of edges with its domain) is its strategy behind the
//! operators' axis driver port.
use super::driver::LeaseDriver;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::service::{element_of, tree_value};
use crate::domain::table_model::types::{SetMembership, Verdict};
use crate::domain::{
    operator::contract::views::IndexedRows, table_cells::types::TreeRows, table_reach::types::Walk,
};
use ryo_derived::domain::membership::types::MembershipView;
use ryo_language::domain::elements::service::{
    children, next_sibling, parent_at, previous_sibling, size_at,
};
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::evaluation::types::{
    Admission, AxisCandidates, AxisDriver, VertexPlace,
};
use ryo_relations::domain::relations::service::{node_file, node_id, node_position};
use ryo_relations::domain::relations::types::{Count, Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
#[derive(Clone, Copy)]
pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: TreeRows<'a>,
        among: Option<TreeRows<'a>>,
        marks: ryo_derived::domain::positions::types::DenseMarks<'a>,
    },
    Indexed {
        edges: IndexedRows<'a>,
        to: ColumnIndex,
        domain: ryo_derived::domain::membership::types::MembershipView<'a>,
    },
}
#[derive(Debug, Default)]
pub(crate) struct AxisWorkspace {
    pub(crate) walk: Walk,
    pub(crate) found: Vec<Position>,
}

/// The axis resources the last walk ran over.
#[derive(Default)]
pub(crate) struct AxisCursor<'f> {
    prepared: Option<PreparedAxis<'f>>,
}

impl LeaseDriver<'_, '_> {
    /// The walk of one axis request over the bound resources, into the workspace's walk or found elements.
    fn walk_axis(
        &mut self,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, OperatingError> {
        let axis = self.resources.axis()?.ok_or(OperatingError::WrongBinding)?;
        self.axis.prepared = Some(axis);
        let work = self
            .lease
            .workspace
            .axis
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        match axis {
            PreparedAxis::Empty => Ok(AxisCandidates::Entities(Count(0))),
            PreparedAxis::Indexed { edges, to, domain } => {
                let start = ryo_values::domain::rows::service::cell(row, id)?;
                work.walk.reached.clear();
                if matches!(direction, Direction::Parent | Direction::Child) {
                    crate::domain::table_reach::borrowed::follow(
                        &edges,
                        to,
                        start,
                        &mut work.walk.key,
                        &mut work.walk.reached,
                    );
                } else if direction == Direction::NearestAncestor {
                    crate::domain::table_reach::borrowed::reach_nearest(
                        &edges,
                        to,
                        start,
                        &mut work.walk,
                        |entity| {
                            let MembershipView::Values(members) = domain else {
                                return Err(OperatingError::WrongBinding);
                            };
                            Ok(if members.contains(&entity) {
                                SetMembership::In
                            } else {
                                SetMembership::Out
                            })
                        },
                    )?;
                } else {
                    crate::domain::table_reach::borrowed::reach_breadth(
                        &edges,
                        to,
                        start,
                        &mut work.walk,
                    );
                }
                Ok(AxisCandidates::Entities(Count(work.walk.reached.len())))
            }
            PreparedAxis::Native { tree, .. } => {
                work.found.clear();
                let Value::Node(start) = ryo_values::domain::rows::service::cell(row, id)? else {
                    return Ok(AxisCandidates::Elements(Count(0)));
                };
                let at = node_position(start);
                let elements = tree.elements;
                if node_file(start) != tree.file || at.0 as usize >= elements.kind.len() {
                    return Ok(AxisCandidates::Elements(Count(0)));
                }
                let found = &mut work.found;
                let up = |from: Position| (from.0 != 0).then(|| parent_at(elements, from));
                match direction {
                    Direction::Parent => found.extend(up(at)),
                    Direction::Child => found.extend(children(elements, at)),
                    Direction::Ancestor | Direction::NearestAncestor => {
                        found.extend(std::iter::successors(up(at), |each| up(*each)));
                    }
                    Direction::Descendant => {
                        found.extend((at.0 + 1..=at.0 + size_at(elements, at).0).map(Position));
                    }
                    Direction::Deep => found.extend(
                        (at.0 + 1..=at.0 + size_at(elements, at).0)
                            .map(Position)
                            .filter(|each| parent_at(elements, *each) != at),
                    ),
                    Direction::PreviousSibling => found.extend(previous_sibling(elements, at)),
                    Direction::NextSibling => found.extend(next_sibling(elements, at)),
                }
                Ok(AxisCandidates::Elements(Count(found.len())))
            }
        }
    }

    /// The element position of a candidate of the last native walk.
    fn element(&self, at: VertexPlace) -> Option<Position> {
        self.lease.workspace.axis.as_ref()?.found.get(at.0).copied()
    }

    /// Whether a candidate of the last walk is admitted.
    fn admit(&mut self, at: VertexPlace) -> Result<Admission, OperatingError> {
        let admitted = match self.axis.prepared.ok_or(OperatingError::WrongBinding)? {
            PreparedAxis::Empty => false,
            PreparedAxis::Indexed { domain, .. } => {
                let entity = self
                    .lease
                    .workspace
                    .axis
                    .as_ref()
                    .and_then(|work| work.walk.reached.get(at.0))
                    .copied()
                    .ok_or(OperatingError::ShapeMismatch)?;
                let MembershipView::Values(members) = domain else {
                    return Err(OperatingError::WrongBinding);
                };
                members.contains(&entity)
            }
            PreparedAxis::Native { tree, among, marks } => {
                let position = self.element(at).ok_or(OperatingError::ShapeMismatch)?;
                if among.is_some() {
                    let reached = among.unwrap_or(tree);
                    self.lease.holds(&element_of(&reached, position))? == Verdict::Holds
                } else {
                    *marks
                        .0
                        .get(position.0 as usize)
                        .ok_or(OperatingError::ShapeMismatch)?
                }
            }
        };
        Ok(if admitted {
            Admission::In
        } else {
            Admission::Out
        })
    }
}

impl AxisDriver for LeaseDriver<'_, '_> {
    fn walk(
        &mut self,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let walked = self.walk_axis(direction, row, id);
        self.hold(walked)
    }
    fn admits(&mut self, at: VertexPlace) -> Result<Admission, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let admitted = self.admit(at);
        self.hold(admitted)
    }
    fn candidate(&self, at: VertexPlace) -> Option<Value> {
        match self.axis.prepared? {
            PreparedAxis::Empty => None,
            PreparedAxis::Indexed { .. } => self
                .lease
                .workspace
                .axis
                .as_ref()?
                .walk
                .reached
                .get(at.0)
                .copied(),
            PreparedAxis::Native { tree, .. } => self
                .element(at)
                .map(|position| Value::Node(node_id(tree.file, position))),
        }
    }
    fn element_cell(&self, at: VertexPlace, column: ColumnIndex) -> Option<Value> {
        let PreparedAxis::Native { tree, among, .. } = self.axis.prepared? else {
            return None;
        };
        let reached = among.unwrap_or(tree);
        self.element(at)
            .map(|position| tree_value(&reached, position, column))
    }
}
