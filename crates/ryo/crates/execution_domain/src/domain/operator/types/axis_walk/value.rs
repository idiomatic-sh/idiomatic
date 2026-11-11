//! The `AxisWalk` value: the axis resources the last walk ran over (a tree of elements, or an index of edges
//! with its domain), the engine's strategy behind the operators' axis port.

use super::super::row_delivery::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::types::LatchState;
use crate::domain::operator::contract::views::types::PreparedAxis;
use crate::domain::table_cells::service::{element_of, tree_value};
use crate::domain::table_model::types::{SetMembership, Verdict};
use ryo_derived::domain::membership::types::MembershipView;
use ryo_language::domain::elements::service::{
    children, next_sibling, parent_at, previous_sibling, size_at,
};
use ryo_operators::domain::axis::types::{AxisAdmission, AxisCandidates};
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::VertexPlace;
use ryo_relations::domain::relations::service::{node_file, node_id, node_position};
use ryo_relations::domain::relations::types::{Count, Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

/// The axis resources the last walk ran over.
#[derive(Default)]
pub(crate) struct AxisWalk<'f> {
    prepared: Option<PreparedAxis<'f>>,
}

impl<'f> AxisWalk<'f> {
    /// The walk of one axis request over the bound resources, into the workspace's walk or found elements.
    fn walk_axis(
        &mut self,
        rows: &mut RowDelivery<'_, 'f>,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, OperatingError> {
        let axis = rows
            .resources()
            .axis()?
            .ok_or(OperatingError::WrongBinding)?;
        self.prepared = Some(axis);
        let work = rows
            .workspace()
            .axis
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        match axis {
            PreparedAxis::Empty => Ok(AxisCandidates::Entities(Count(0))),
            PreparedAxis::Indexed { edges, to, domain } => {
                let start = ryo_values::domain::rows::service::cell(row, id)?;
                work.walk.reached.clear();
                if matches!(direction, Direction::Parent | Direction::Child) {
                    crate::domain::table_reach::borrowed::service::follow(
                        &edges,
                        to,
                        start,
                        &mut work.walk.key,
                        &mut work.walk.reached,
                    );
                } else if direction == Direction::NearestAncestor {
                    crate::domain::table_reach::borrowed::service::reach_nearest(
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
                    crate::domain::table_reach::borrowed::service::reach_breadth(
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
                if node_file(start) != tree.file
                    || !usize::try_from(at.0).is_ok_and(|place| place < elements.kind.len())
                {
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
    fn element(rows: &RowDelivery<'_, '_>, at: VertexPlace) -> Option<Position> {
        rows.workspace_view()
            .axis
            .as_ref()?
            .found
            .get(at.0)
            .copied()
    }

    fn admit(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        at: VertexPlace,
    ) -> Result<AxisAdmission, OperatingError> {
        let admitted = match self.prepared.ok_or(OperatingError::WrongBinding)? {
            PreparedAxis::Empty => false,
            PreparedAxis::Indexed { domain, .. } => {
                let entity = rows
                    .workspace_view()
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
                let position = Self::element(rows, at).ok_or(OperatingError::ShapeMismatch)?;
                if among.is_some() {
                    let reached = among.unwrap_or(tree);
                    rows.verdict(&element_of(&reached, position))? == Verdict::Holds
                } else {
                    let Ok(slot) = usize::try_from(position.0) else {
                        return Err(OperatingError::ShapeMismatch);
                    };
                    *marks.0.get(slot).ok_or(OperatingError::ShapeMismatch)?
                }
            }
        };
        Ok(if admitted {
            AxisAdmission::In
        } else {
            AxisAdmission::Out
        })
    }

    /// Walks one axis request from the row's `id` cell.
    pub(crate) fn walk(
        &mut self,
        rows: &mut RowDelivery<'_, 'f>,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let walked = self.walk_axis(rows, direction, row, id);
        rows.hold(walked)
    }

    /// Whether a candidate of the last walk is admitted.
    pub(crate) fn admits(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        at: VertexPlace,
    ) -> Result<AxisAdmission, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let admitted = self.admit(rows, at);
        rows.hold(admitted)
    }

    /// One candidate of the last walk.
    pub(crate) fn candidate(&self, rows: &RowDelivery<'_, '_>, at: VertexPlace) -> Option<Value> {
        match self.prepared? {
            PreparedAxis::Empty => None,
            PreparedAxis::Indexed { .. } => rows
                .workspace_view()
                .axis
                .as_ref()?
                .walk
                .reached
                .get(at.0)
                .copied(),
            PreparedAxis::Native { tree, .. } => {
                Self::element(rows, at).map(|position| Value::Node(node_id(tree.file, position)))
            }
        }
    }

    /// One cell of a native candidate of the last walk.
    pub(crate) fn element_cell(
        &self,
        rows: &RowDelivery<'_, '_>,
        at: VertexPlace,
        column: ColumnIndex,
    ) -> Option<Value> {
        let PreparedAxis::Native { tree, among, .. } = self.prepared? else {
            return None;
        };
        let reached = among.unwrap_or(tree);
        Self::element(rows, at).map(|position| tree_value(&reached, position, column))
    }
}
