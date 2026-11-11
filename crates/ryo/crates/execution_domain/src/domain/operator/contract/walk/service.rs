//! The `walk` service: free functions named by verb over the domain's types, with no IO and no ports.
use crate::domain::operator::contract::driver::service::hold;
use crate::domain::operator::contract::driver::service::interrupted;

use super::super::driver::types::LeaseDriver;
use super::super::views::types::PreparedAxis;
use crate::domain::operating::errors::OperatingError;
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

/// The walk of one axis request over the bound resources, into the workspace's walk or found elements.
pub(crate) fn walk_axis(
    driver: &mut LeaseDriver<'_, '_>,
    direction: Direction,
    row: &[Value],
    id: ColumnIndex,
) -> Result<AxisCandidates, OperatingError> {
    let axis = driver
        .resources
        .axis()?
        .ok_or(OperatingError::WrongBinding)?;
    driver.axis.prepared = Some(axis);
    let work = driver
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
pub(crate) fn element(driver: &LeaseDriver<'_, '_>, at: VertexPlace) -> Option<Position> {
    driver
        .lease
        .workspace
        .axis
        .as_ref()?
        .found
        .get(at.0)
        .copied()
}

/// Whether a candidate of the last walk is admitted.
pub(crate) fn admit(
    driver: &mut LeaseDriver<'_, '_>,
    at: VertexPlace,
) -> Result<AxisAdmission, OperatingError> {
    let admitted = match driver.axis.prepared.ok_or(OperatingError::WrongBinding)? {
        PreparedAxis::Empty => false,
        PreparedAxis::Indexed { domain, .. } => {
            let entity = driver
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
            let position = element(driver, at).ok_or(OperatingError::ShapeMismatch)?;
            if among.is_some() {
                let reached = among.unwrap_or(tree);
                crate::domain::operator::contract::lease::service::holds(
                    driver.lease,
                    &element_of(&reached, position),
                )? == Verdict::Holds
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

pub(crate) fn walk(
    driver: &mut LeaseDriver<'_, '_>,
    direction: Direction,
    row: &[Value],
    id: ColumnIndex,
) -> Result<AxisCandidates, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let walked = walk_axis(driver, direction, row, id);
    hold(driver, walked)
}

pub(crate) fn admits(
    driver: &mut LeaseDriver<'_, '_>,
    at: VertexPlace,
) -> Result<AxisAdmission, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let admitted = admit(driver, at);
    hold(driver, admitted)
}

pub(crate) fn candidate(driver: &LeaseDriver<'_, '_>, at: VertexPlace) -> Option<Value> {
    match driver.axis.prepared? {
        PreparedAxis::Empty => None,
        PreparedAxis::Indexed { .. } => driver
            .lease
            .workspace
            .axis
            .as_ref()?
            .walk
            .reached
            .get(at.0)
            .copied(),
        PreparedAxis::Native { tree, .. } => {
            element(driver, at).map(|position| Value::Node(node_id(tree.file, position)))
        }
    }
}

pub(crate) fn element_cell(
    driver: &LeaseDriver<'_, '_>,
    at: VertexPlace,
    column: ColumnIndex,
) -> Option<Value> {
    let PreparedAxis::Native { tree, among, .. } = driver.axis.prepared? else {
        return None;
    };
    let reached = among.unwrap_or(tree);
    element(driver, at).map(|position| tree_value(&reached, position, column))
}
