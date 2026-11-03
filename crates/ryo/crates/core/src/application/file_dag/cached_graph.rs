//! Generation-owned complete graph acquisition and one-row demand answers.
//! The primitive sees only borrowed values after the mutable cache borrow ends.
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::derived::graph_cache::types::handles::{
    DomainHandle, GraphHandle,
};
use crate::application::file_dag::derived::graph_cache::types::roots::value::DemandAnswers;
use crate::application::file_dag::derived::owner::DerivedOwner;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    graph_view::{AxisResult, CachedGraphRow, ReachResult},
    resources::ResourceProgram,
};
use crate::domain::stored_graph::{
    types::traversal::value::GraphTraversal,
    types::{GraphWalk, Orientation},
};
use crate::domain::table_model::{service::cell, types::Rows};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

/// One request-bound plan with invocation-local traversal buffers. Exact cache
/// identity and bounded retention remain in `CacheBackend` and `graph_cache`.
pub(crate) enum CachedGraph {
    Reach(ReachGraph),
    Axis(AxisGraph),
}

pub(crate) struct ReachGraph {
    access: GraphHandle,
    left_on: Vec<ColumnIndex>,
    bindings: Vec<usize>,
    partition_width: usize,
    key: Vec<Value>,
    request: Vec<Value>,
    destinations: Vec<Value>,
    walk: GraphWalk,
}

pub(crate) struct AxisGraph {
    access: GraphHandle,
    domain: DomainHandle,
    id: ColumnIndex,
    direction: Direction,
    destinations: Vec<Value>,
    walk: GraphWalk,
}

impl CachedGraph {
    /// Called only after the common engine has held and checked the needed
    /// inputs at the operation's declared lazy acquisition boundary. Graph
    /// ineligibility returns `None` so the ordinary/native typed backend runs.
    pub(in crate::application::file_dag) fn prepare(
        contract: &crate::domain::operator::contract::types::OperatorContract,
        admission: &super::workspace::InvocationWorkspace,
        mode: crate::domain::operator::contract::types::EvaluationMode,
        program: &ResourceProgram,
        original_inputs: &[Rows<'_>],
        mut cache: Option<&mut CacheBackend>,
    ) -> Result<Option<Self>, OperatingError> {
        use crate::domain::derived_views::service::slot;
        use crate::domain::operator::contract::types::EvaluationMode;
        use ryo_derived::domain::contract::types::DerivedKind;
        let slots = match mode {
            EvaluationMode::Streaming => &contract.derived.streaming,
            EvaluationMode::Collected => &contract.derived.collected,
        };
        let adjacency = || {
            slots[slot(DerivedKind::Adjacency)]
                .as_ref()
                .map(|group| &group.primary.plan)
                .ok_or(OperatingError::NotRun)
        };
        match program {
            ResourceProgram::Join(join) => {
                let Some(reach) = join.reach.as_ref() else {
                    return Ok(None);
                };
                if join.has_filter
                    || !join.view.is_empty()
                    || join.left_on.len() != join.right_on.len()
                {
                    return Ok(None);
                }
                let bindings = (0..=reach.keys.len())
                    .map(|logical| {
                        join.right_on.iter().position(|column| {
                            reach.columns.get(column.0 as usize)
                                == Some(&ColumnIndex(u32::try_from(logical).unwrap_or(u32::MAX)))
                        })
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(bindings) = bindings else {
                    return Ok(None);
                };
                let Some(rows) = original_inputs.get(1) else {
                    return Ok(None);
                };
                if !matches!(rows, Rows::Stored(_) | Rows::Selected(_)) {
                    return Ok(None);
                }
                let Some(source) = admission.graph_input(1, rows)? else {
                    return Ok(None);
                };
                let Some(access) =
                    source
                        .owner()
                        .acquire_graph(adjacency()?, &source, cache.as_deref_mut())?
                else {
                    return Ok(None);
                };
                Ok(Some(Self::Reach(ReachGraph {
                    access,
                    left_on: join.left_on.clone(),
                    bindings,
                    partition_width: reach.keys.len(),
                    key: Vec::with_capacity(join.left_on.len()),
                    request: Vec::with_capacity(reach.keys.len() + 1),
                    destinations: Vec::new(),
                    walk: GraphWalk::default(),
                })))
            }
            ResourceProgram::Axis(axis) => {
                if axis.has_filter
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
                if !matches!(edges, Rows::Stored(_) | Rows::Selected(_)) {
                    return Ok(None);
                }
                let Some(source) = admission.graph_input(1, edges)? else {
                    return Ok(None);
                };
                let Some(access) =
                    source
                        .owner()
                        .acquire_graph(adjacency()?, &source, cache.as_deref_mut())?
                else {
                    return Ok(None);
                };
                let Some(domain_input) = admission.graph_input(2, domain_rows)? else {
                    return Ok(None);
                };
                let membership = domain_input.owner().acquire_graph_domain(
                    &slots[slot(DerivedKind::Membership)]
                        .as_ref()
                        .ok_or(OperatingError::NotRun)?
                        .primary
                        .plan,
                    &access,
                    &domain_input,
                    cache.as_deref_mut(),
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
            _ => Ok(None),
        }
    }

    /// A completed demand result borrows only this invocation's value buffers.
    /// Root and nearest memo admission occurs before a downstream row can emit.
    pub(in crate::application::file_dag) fn row<'a>(
        &'a mut self,
        left: &[Value],
        owner: &DerivedOwner,
        cache: Option<&mut CacheBackend>,
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        match self {
            Self::Reach(reach) => Ok(CachedGraphRow::Reach(reach.row(left, owner, cache)?)),
            Self::Axis(axis) => Ok(CachedGraphRow::Axis(axis.row(left, owner, cache)?)),
        }
    }
}

impl ReachGraph {
    fn row<'a>(
        &'a mut self,
        left: &[Value],
        owner: &DerivedOwner,
        cache: Option<&mut CacheBackend>,
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
                    .get(*position)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        self.destinations.clear();
        let source = *self
            .request
            .get(self.partition_width)
            .ok_or(OperatingError::ShapeMismatch)?;
        let reached = DemandAnswers::new(owner, cache).reach(
            &self.access,
            &self.request[..self.partition_width],
            source,
            &mut self.walk,
        )?;
        if let Some(answer) = reached {
            for value in answer.values()? {
                self.destinations.push(value?);
            }
        } else {
            for vertex in &self.walk.reached {
                self.destinations.push(
                    *self
                        .access
                        .graph
                        .vertices
                        .get(*vertex as usize)
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
        mut cache: Option<&mut CacheBackend>,
    ) -> Result<AxisResult<'a>, OperatingError> {
        self.destinations.clear();
        let start = cell(left, self.id)?;
        let Some(root) = self.access.graph.vertex_ids.get(&(0, start)).copied() else {
            return Ok(AxisResult {
                destinations: &self.destinations,
            });
        };
        let orientation = match self.direction {
            Direction::Child | Direction::Descendant => self.access.orientation,
            Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => {
                match self.access.orientation {
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
            let found = DemandAnswers::new(owner, cache.as_deref_mut()).nearest(
                &self.access,
                &self.domain,
                root,
                orientation,
                &mut answers,
                &mut self.walk,
            )?;
            self.walk.answers = answers;
            if let Some(found) = found {
                self.destinations.push(
                    *self
                        .access
                        .graph
                        .vertices
                        .get(found as usize)
                        .ok_or(OperatingError::ShapeMismatch)?,
                );
            }
        } else {
            let mut traversal = GraphTraversal::new(&self.access.graph, &mut self.walk);
            if matches!(self.direction, Direction::Parent | Direction::Child) {
                traversal.direct(root, orientation);
            } else {
                let _ = traversal.breadth(root, orientation, None);
            }
            if let Some(cache) = cache.as_deref_mut() {
                cache.observe_graph_walk(self.walk.expanded, self.walk.examined);
            }
            for vertex in &self.walk.reached {
                if self.domain.members.get(*vertex as usize) == Some(&true) {
                    self.destinations.push(
                        *self
                            .access
                            .graph
                            .vertices
                            .get(*vertex as usize)
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
