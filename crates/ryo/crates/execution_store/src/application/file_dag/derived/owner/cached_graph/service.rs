//! Preparation and rows of the graph demand; the owner cache is required for both.
use super::types::{AxisGraph, CachedGraph, KeyPosition, PartitionWidth, ReachGraph};
use crate::application::file_dag::derived::owner::DerivedOwner;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
use ryo_derived::domain::vertices::types::VertexId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::{
    graph_view::{AxisResult, CachedGraphRow, ReachResult},
    resources::ResourceProgram,
};
use ryo_execution_domain::domain::stored_graph::types::{GraphWalk, Orientation};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
use ryo_values::domain::rows::service::cell;

impl CachedGraph {
    /// Called only after the common engine has held and checked the needed
    /// inputs at the operation's declared lazy acquisition boundary. A program the graph
    /// route cannot serve returns `None` so the ordinary/native typed backend runs; the
    /// owner cache is required and never decides the route.
    pub(crate) fn prepare(
        contract: &ryo_execution_domain::domain::operator::contract::types::OperatorContract,
        admission: &crate::application::file_dag::workspace::InvocationWorkspace,
        program: &ResourceProgram,
        original_inputs: &[Rows<'_>],
        cache: &mut CacheBackend,
    ) -> Result<Option<Self>, OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        let slots = &contract.derived.slots;
        let adjacency = || {
            slots[DerivedKind::Adjacency.ordinal()]
                .as_ref()
                .map(|group| &group.primary.plan)
                .ok_or(OperatingError::NotRun)
        };
        match program {
            ResourceProgram::Join(join) => {
                let Some(reach) = join.reach.as_ref() else {
                    return Ok(None);
                };
                if join.has_filter == ryo_execution_domain::domain::operator::contract::resources::Filtering::Filtered
                    || !join.view.is_empty()
                    || join.left_on.len() != join.right_on.len()
                {
                    return Ok(None);
                }
                let bindings = (0..=reach.keys.len())
                    .map(|logical| {
                        join.right_on
                            .iter()
                            .position(|column| {
                                usize::try_from(column.0).is_ok_and(|at| {
                                    reach.columns.get(at)
                                        == Some(&ColumnIndex(
                                            u32::try_from(logical).unwrap_or(u32::MAX),
                                        ))
                                })
                            })
                            .map(KeyPosition)
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(bindings) = bindings else {
                    return Ok(None);
                };
                let Some(rows) = original_inputs.get(1) else {
                    return Ok(None);
                };
                if !matches!(rows, Rows::Stored(_)) {
                    return Ok(None);
                }
                let Some(source) = admission.graph_input(InputNumber(1), rows)? else {
                    return Ok(None);
                };
                let Some(access) =
                    source
                        .owner()
                        .acquire_graph(adjacency()?, &source, &mut *cache)?
                else {
                    return Ok(None);
                };
                Ok(Some(Self::Reach(ReachGraph {
                    access,
                    left_on: join.left_on.clone(),
                    bindings,
                    partition_width: PartitionWidth(reach.keys.len()),
                    key: Vec::with_capacity(join.left_on.len()),
                    request: Vec::with_capacity(reach.keys.len() + 1),
                    destinations: Vec::new(),
                    walk: GraphWalk::default(),
                })))
            }
            ResourceProgram::Axis(axis) => {
                if axis.has_filter == ryo_execution_domain::domain::operator::contract::resources::Filtering::Filtered
                    || !axis.domain_view.is_empty()
                    || !axis.output.is_empty()
                    || matches!(
                        axis.direction,
                        Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
                    )
                {
                    return Ok(None);
                }
                let (Some(edges), Some(domain_rows)) =
                    (original_inputs.get(1), original_inputs.get(2))
                else {
                    return Ok(None);
                };
                if !matches!(edges, Rows::Stored(_)) {
                    return Ok(None);
                }
                let Some(source) = admission.graph_input(InputNumber(1), edges)? else {
                    return Ok(None);
                };
                let Some(access) =
                    source
                        .owner()
                        .acquire_graph(adjacency()?, &source, &mut *cache)?
                else {
                    return Ok(None);
                };
                let Some(domain_input) = admission.graph_input(InputNumber(2), domain_rows)? else {
                    return Ok(None);
                };
                let membership = domain_input.owner().acquire_graph_domain(
                    &slots[DerivedKind::Membership.ordinal()]
                        .as_ref()
                        .ok_or(OperatingError::NotRun)?
                        .primary
                        .plan,
                    &access,
                    &domain_input,
                    &mut *cache,
                )?;
                Ok(Some(Self::Axis(AxisGraph {
                    access,
                    domain: membership,
                    id: axis.id,
                    direction: axis.direction,
                    destinations: Vec::new(),
                    walk: GraphWalk::default(),
                })))
            }
            ResourceProgram::None
            | ResourceProgram::Appended(_)
            | ResourceProgram::Rank(_)
            | ResourceProgram::Index { .. }
            | ResourceProgram::Count { .. }
            | ResourceProgram::Digest { .. }
            | ResourceProgram::Closure { .. }
            | ResourceProgram::Cycles { .. }
            | ResourceProgram::Ordered { .. }
            | ResourceProgram::Dfs(_) => Ok(None),
        }
    }

    /// A completed demand result borrows only this invocation's value buffers.
    /// Root and nearest memo admission occurs before a downstream row can emit.
    pub(crate) fn row<'a>(
        &'a mut self,
        left: &[Value],
        owner: &DerivedOwner,
        cache: &mut CacheBackend,
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        match self {
            Self::Reach(reach) => {
                reach.access.validate_owner(owner)?;
                Ok(CachedGraphRow::Reach(reach.row(left, owner, cache)?))
            }
            Self::Axis(axis) => {
                axis.access.validate_owner(owner)?;
                axis.domain.validate_owner(owner, &axis.access)?;
                Ok(CachedGraphRow::Axis(axis.row(left, owner, cache)?))
            }
        }
    }
}

impl ReachGraph {
    fn row<'a>(
        &'a mut self,
        left: &[Value],
        owner: &DerivedOwner,
        cache: &mut CacheBackend,
    ) -> Result<ReachResult<'a>, OperatingError> {
        self.key.clear();
        for column in &self.left_on {
            self.key.push(cell(left, *column)?);
        }
        self.request.clear();
        for position in &self.bindings {
            self.request.push(
                *self
                    .key
                    .get(position.0)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        self.destinations.clear();
        let source = *self
            .request
            .get(self.partition_width.0)
            .ok_or(OperatingError::ShapeMismatch)?;
        let reached = DemandAnswers::new(owner, cache).reach(
            &self.access,
            &self.request[..self.partition_width.0],
            source,
            &mut self.walk,
        )?;
        if let Some(answer) = reached {
            for value in answer.values()? {
                self.destinations.push(value?);
            }
        } else {
            for vertex in &self.walk.reached {
                let Ok(at) = usize::try_from(vertex.0) else {
                    return Err(OperatingError::ShapeMismatch);
                };
                self.destinations.push(
                    *self
                        .access
                        .graph()
                        .vertices()
                        .get(at)
                        .ok_or(OperatingError::ShapeMismatch)?,
                );
            }
        }
        Ok(ReachResult {
            request: &self.request,
            destinations: &self.destinations,
        })
    }
}

impl AxisGraph {
    fn row<'a>(
        &'a mut self,
        left: &[Value],
        owner: &DerivedOwner,
        cache: &mut CacheBackend,
    ) -> Result<AxisResult<'a>, OperatingError> {
        self.destinations.clear();
        let start = cell(left, self.id)?;
        let Some(root) = self
            .access
            .graph()
            .vertex(ryo_derived::domain::vertices::types::PartitionId(0), start)
            .map(|vertex| vertex.0)
        else {
            return Ok(AxisResult {
                destinations: &self.destinations,
            });
        };
        let orientation = match self.direction {
            Direction::Child | Direction::Descendant => self.access.orientation(),
            Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => {
                match self.access.orientation() {
                    Orientation::Forward => Orientation::Reverse,
                    Orientation::Reverse => Orientation::Forward,
                }
            }
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling => {
                return Err(OperatingError::WrongOperation);
            }
        };
        if self.direction == Direction::NearestAncestor {
            let mut answers = std::mem::take(&mut self.walk.answers);
            let found = DemandAnswers::new(owner, &mut *cache).nearest(
                &self.access,
                &self.domain,
                VertexId(root),
                orientation,
                &mut answers,
                &mut self.walk,
            )?;
            self.walk.answers = answers;
            if let Some(found) = found {
                let Ok(at) = usize::try_from(found.0) else {
                    return Err(OperatingError::ShapeMismatch);
                };
                self.destinations.push(
                    *self
                        .access
                        .graph()
                        .vertices()
                        .get(at)
                        .ok_or(OperatingError::ShapeMismatch)?,
                );
            }
        } else {
            DemandAnswers::new(owner, &mut *cache).axis(
                &self.access,
                VertexId(root),
                orientation,
                self.direction,
                &mut self.walk,
            )?;
            for vertex in &self.walk.reached {
                let Ok(at) = usize::try_from(vertex.0) else {
                    return Err(OperatingError::ShapeMismatch);
                };
                if self.domain.members().0.get(at) == Some(&true) {
                    self.destinations.push(
                        *self
                            .access
                            .graph()
                            .vertices()
                            .get(at)
                            .ok_or(OperatingError::ShapeMismatch)?,
                    );
                }
            }
        }
        Ok(AxisResult {
            destinations: &self.destinations,
        })
    }
}
