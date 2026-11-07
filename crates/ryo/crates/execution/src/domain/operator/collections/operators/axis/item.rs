//! Axis traversal semantics over engine-prepared immutable tree/domain resources.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::axis::PreparedAxis;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_cells::service::element_of;
use crate::domain::table_reach::borrowed::{follow, reach_breadth, reach_nearest};
use crate::domain::tree_join::service::driver::tree_value;
use ryo_derived::domain::membership::types::MembershipView;
use ryo_language::domain::elements::service::{
    children, next_sibling, parent_at, previous_sibling, size_at,
};
use ryo_relations::domain::relations::{
    service::{node_file, node_id, node_position},
    types::{Position, Value},
};
use ryo_values::domain::plan_parts::types::{Direction, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use {
    crate::domain::table_model::types::{SetMembership, Verdict},
    ryo_values::domain::rows::service::cell,
};
pub(crate) struct AxisItem;
impl OperatorsItem for AxisItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Axis,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let Operation::Axis {
            id,
            direction,
            output,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        if let Some(cached) = lease.views.cached_axis {
            let mut answer = std::mem::take(&mut lease.workspace.row);
            let result = (|| {
                for destination in cached.destinations {
                    answer.clear();
                    answer.extend_from_slice(row);
                    answer.push(*destination);
                    emit.row(lease, &answer)?;
                }
                Ok(())
            })();
            lease.workspace.row = answer;
            return result;
        }
        let bound_axis = lease.views.resources.axis()?;
        let axis = bound_axis.as_ref().ok_or(OperatingError::WrongBinding)?;
        let mut answer = std::mem::take(&mut lease.workspace.row);
        let result = (|| match axis {
            PreparedAxis::Empty => Ok(()),
            PreparedAxis::Indexed { edges, to, domain } => {
                let start = cell(row, *id)?;
                let work = lease
                    .workspace
                    .axis
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?;
                work.walk.reached.clear();
                if matches!(direction, Direction::Parent | Direction::Child) {
                    follow(
                        edges,
                        *to,
                        start,
                        &mut work.walk.key,
                        &mut work.walk.reached,
                    );
                } else if *direction == Direction::NearestAncestor {
                    reach_nearest(edges, *to, start, &mut work.walk, |entity| {
                        let MembershipView::Values(members) = domain else {
                            return Err(OperatingError::WrongBinding);
                        };
                        Ok(if members.contains(&entity) {
                            SetMembership::In
                        } else {
                            SetMembership::Out
                        })
                    })?;
                } else {
                    reach_breadth(edges, *to, start, &mut work.walk);
                }
                let count = work.walk.reached.len();
                for at in 0..count {
                    let entity = lease
                        .workspace
                        .axis
                        .as_ref()
                        .and_then(|work| work.walk.reached.get(at))
                        .copied()
                        .ok_or(OperatingError::ShapeMismatch)?;
                    let MembershipView::Values(members) = domain else {
                        return Err(OperatingError::WrongBinding);
                    };
                    if !members.contains(&entity) {
                        continue;
                    }
                    answer.clear();
                    answer.extend_from_slice(row);
                    answer.push(entity);
                    emit.row(lease, &answer)?;
                    if *direction == Direction::NearestAncestor {
                        break;
                    }
                }
                Ok(())
            }
            PreparedAxis::Native { tree, among, marks } => {
                let Value::Node(start) = cell(row, *id)? else {
                    return Ok(());
                };
                let at = node_position(start);
                let elements = tree.elements;
                if node_file(start) != tree.file || at.0 as usize >= elements.kind.len() {
                    return Ok(());
                }
                let mut found = std::mem::take(
                    &mut lease
                        .workspace
                        .axis
                        .as_mut()
                        .ok_or(OperatingError::WrongBinding)?
                        .found,
                );
                found.clear();
                let up = |from: Position| (from.0 != 0).then(|| parent_at(elements, from));
                match direction {
                    Direction::Parent => found.extend(up(at)),
                    Direction::Child => found.extend(children(elements, at)),
                    Direction::Ancestor | Direction::NearestAncestor => {
                        found.extend(std::iter::successors(up(at), |each| up(*each)))
                    }
                    Direction::Descendant => {
                        found.extend((at.0 + 1..=at.0 + size_at(elements, at).0).map(Position))
                    }
                    Direction::Deep => found.extend(
                        (at.0 + 1..=at.0 + size_at(elements, at).0)
                            .map(Position)
                            .filter(|each| parent_at(elements, *each) != at),
                    ),
                    Direction::PreviousSibling => {
                        found.extend(previous_sibling(elements, at))
                    }
                    Direction::NextSibling => {
                        found.extend(next_sibling(elements, at))
                    }
                }
                let reached = among.unwrap_or(*tree);
                let outcome = (|| {
                    for each in &found {
                        let position = *each;
                        let admitted = if among.is_some() {
                            lease.holds(&element_of(&reached, position))? == Verdict::Holds
                        } else {
                            *marks
                                .0
                                .get(each.0 as usize)
                                .ok_or(OperatingError::ShapeMismatch)?
                        };
                        if !admitted {
                            continue;
                        }
                        answer.clear();
                        if output.is_empty() {
                            answer.extend_from_slice(row);
                            answer.push(Value::Node(node_id(tree.file, position)));
                        } else {
                            for column in output {
                                answer.push(match column.side {
                                    Side::Left => cell(row, column.column)?,
                                    Side::Right => tree_value(&reached, position, column.column),
                                });
                            }
                        }
                        emit.row(lease, &answer)?;
                        if *direction == Direction::NearestAncestor {
                            break;
                        }
                    }
                    Ok(())
                })();
                lease
                    .workspace
                    .axis
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?
                    .found = found;
                outcome
            }
        })();
        lease.workspace.row = answer;
        result
    }
}
