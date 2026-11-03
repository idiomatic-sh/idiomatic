//! Demand a unique dependency schedule; primitive semantics stay in domain.
use super::executor::PrimitiveExecutor;
use super::interface::DagExecution;
use super::types::FileDagExecution;
use crate::application::file_dag::feed::Feed;
use crate::domain::file_program::types::{NodeId, NodeInput, RequestGraph};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Room, Supply};
use crate::domain::replay::service::scan;
use crate::domain::replay::types::Collect;
use crate::domain::table_model::service::reset;
use crate::domain::table_model::types::{RowCount, Rows, Table};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use std::cell::OnceCell;
use std::sync::Arc;

/// Retained result buffers only; injected typed indexes keep their existing allowance.
const RESULT_LIMIT: usize = 6 * 1024 * 1024;
pub(super) fn table_bytes(table: &Table) -> usize {
    table
        .cells
        .capacity()
        .saturating_mul(size_of::<Value>())
        .saturating_add(table.kinds.capacity().saturating_mul(size_of::<CellKind>()))
        .saturating_add(size_of::<Table>() + 2 * size_of::<usize>())
}
pub(super) use super::relations::bytes;
impl FileDagExecution {
    #[cfg(test)]
    pub(crate) fn new(
        graph: Arc<RequestGraph>,
        active: impl Fn(usize) -> bool,
        complete: &[Relation],
    ) -> Self {
        Self::new_for_population(graph, active, complete, true)
    }
    pub(crate) fn new_for_population(
        graph: Arc<RequestGraph>,
        active: impl Fn(usize) -> bool,
        complete: &[Relation],
        stable: bool,
    ) -> Self {
        let active = graph
            .cuts
            .iter()
            .enumerate()
            .map(|(rule, cuts)| {
                active(rule)
                    && cuts.iter().flatten().all(|node| {
                        graph.nodes[node.0].schema_valid
                            && graph.nodes[node.0]
                                .relations
                                .iter()
                                .all(|relation| complete.contains(relation))
                    })
            })
            .collect::<Vec<_>>();
        let mut needed = vec![false; graph.nodes.len()];
        let mut remaining = vec![0_usize; graph.nodes.len()];
        let mut pending = Vec::new();
        for (rule, roots) in graph.boundaries.iter().enumerate() {
            if !active[rule] {
                continue;
            }
            for (_, root) in roots {
                remaining[root.0] += 1;
                pending.push(*root);
            }
        }
        while let Some(node) = pending.pop() {
            if needed[node.0] {
                continue;
            }
            needed[node.0] = true;
            for input in &graph.nodes[node.0].inputs {
                if let NodeInput::Node(child) = input {
                    remaining[child.0] += 1;
                    pending.push(*child);
                }
            }
        }
        let count = graph.nodes.len();
        Self {
            relations: std::rc::Rc::new(super::relations::RelationOwner::for_request(
                stable,
                graph.nodes.len() + complete.len() + 1,
            )),
            graph,
            active,
            remaining,
            ready: vec![None; count],
            required: vec![None; count],
            required_remaining: vec![0; count],
            required_executed: vec![false; count],
            required_inputs: vec![false; count],
            required_nodes: Vec::new(),
            current_rule: None,
            required_bytes: 0,
            peak_required_bytes: 0,
            overlap_bytes: 0,
            peak_owned_bytes: 0,
            required_releases: 0,
            visiting: vec![false; count],
            executions: vec![0; count],
            demands: 0,
            hits: 0,
            releases: 0,
            retained_bytes: 0,
            peak_bytes: 0,
            limit: RESULT_LIMIT,
            unretained: 0,
        }
    }
    fn consumed(&mut self, node: NodeId) {
        if self.remaining[node.0] == 0 {
            return;
        }
        self.remaining[node.0] -= 1;
        if self.remaining[node.0] != 0 {
            return;
        }
        if self.executions[node.0] > 0 && self.graph.derived_sources[node.0] {
            self.relations.release_derived_producer(node);
        }
        if let Some(table) = self.ready[node.0].take() {
            let size = bytes(&table);
            self.retained_bytes = self.retained_bytes.saturating_sub(size);
            if self.required[node.0].is_some() {
                self.overlap_bytes = self.overlap_bytes.saturating_sub(size);
            }
            self.releases += 1;
        }
        // A lazy primitive may never request one branch. Cancel that branch's
        // outgoing consumer edges when it becomes unreachable, not at file end.
        if self.executions[node.0] == 0 {
            let graph = (self.graph).clone();
            for input in &graph.nodes[node.0].inputs {
                if let NodeInput::Node(child) = input {
                    self.consumed(*child);
                }
            }
        }
    }
    pub(crate) fn finish_rule(&mut self, rule: usize) {
        if !self.active.get(rule).copied().unwrap_or(false) {
            return;
        }
        let graph = (self.graph).clone();
        for (_, root) in &graph.boundaries[rule] {
            self.consumed(*root);
            self.consume_required(*root);
        }
        self.clear_required();
    }
}
impl FileDagExecution {
    pub(crate) fn owned(&self, rule: usize, step: usize) -> Option<NodeId> {
        self.active
            .get(rule)
            .copied()
            .unwrap_or(false)
            .then(|| self.graph.cuts.get(rule)?.get(step).copied().flatten())
            .flatten()
    }
    fn demand_result<'f>(
        &mut self,
        id: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &dyn PrimitiveExecutor,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        let _node = crate::application::file_dag::route_counts::node(id.0);
        self.demands += 1;
        if let Some(table) = self.required.get(id.0).and_then(Option::as_ref) {
            self.hits += 1;
            return Ok((table).clone());
        }
        if let Some(table) = self
            .ready
            .get(id.0)
            .and_then(Option::as_ref)
            .map(Clone::clone)
        {
            self.hits += 1;
            let table = self.relations.required_publication(&table)?;
            self.pin_required(id, table.clone());
            return Ok(table);
        }
        if self.visiting.get(id.0).copied().unwrap_or(true) {
            return Err(OperatingError::NotRun);
        }
        let graph = (self.graph).clone();
        let node = &graph.nodes[id.0];
        if crate::domain::features::service::kind_of(&node.operation) != node.contract.kind {
            return Err(OperatingError::WrongOperation);
        }
        self.visiting[id.0] = true;
        let held = (0..node.inputs.len())
            .map(|_| OnceCell::new())
            .collect::<Vec<_>>();
        let owner = std::rc::Rc::clone(&self.relations);
        let mut feed = GraphInputs {
            execution: self,
            inputs: &node.inputs,
            schemas: &node.contract.input_kinds,
            held: &held,
            read,
            executor,
        };
        let outcome = executor.keep_representation(
            &node.contract,
            &node.physical,
            &mut feed,
            &owner,
            context,
        );
        drop(feed);
        drop(held);
        self.visiting[id.0] = false;
        let table = outcome?;
        if self.graph.derived_sources[id.0] {
            self.relations.bind_producer(id, table.as_ref())?;
        }
        let first_global_execution = self.executions[id.0] == 0;
        self.executions[id.0] += 1;
        // A global edge is consumed once, not once per optional-cache refusal.
        // Local edges represent the actual required rule lifetime independently.
        let first_required_execution = !self.required_executed[id.0];
        self.required_executed[id.0] = true;
        for input in &node.inputs {
            if let NodeInput::Node(child) = input {
                if first_global_execution {
                    self.consumed(*child);
                }
                if first_required_execution && self.required_inputs[id.0] {
                    self.consume_required(*child);
                }
            }
        }
        self.pin_required(id, (table).clone());
        // Global consumers own the derived population independently of whether
        // this producer's Table fits the optional result cache. Rule-local
        // required pins remain a separate, symmetric lifetime.
        if first_global_execution && self.remaining[id.0] > 0 && self.graph.derived_sources[id.0] {
            self.relations.pin_derived(table.as_ref());
        }
        let size = bytes(&table);
        let retained = if self.remaining[id.0] > 0 {
            self.relations.retain_publication(&table, self.limit)?
        } else {
            None
        };
        if let Some(retained) = retained {
            self.retained_bytes += size;
            self.peak_bytes = self.peak_bytes.max(self.retained_bytes);
            self.ready[id.0] = Some(retained);
            if self.required[id.0].is_some() {
                self.overlap_bytes = self.overlap_bytes.saturating_add(size);
            }
            self.observe_required();
        } else {
            // Explicit budget fallback: later demand may recompute total pure work.
            self.unretained += 1;
        }
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "dag_node",
            node = id.0, executions = self.executions[id.0], representation=?crate::domain::table_model::collections::representations::publication::PublishedRelation::kind(table.as_ref()),
            retained_bytes = self.retained_bytes, peak_bytes = self.peak_bytes,
            required_bytes=self.required_bytes, peak_required_bytes=self.peak_required_bytes,
            overlap_bytes=self.overlap_bytes, peak_owned_bytes=self.peak_owned_bytes,
            required_releases=self.required_releases,
            required_metadata_bytes=self.required_metadata_bytes(),
            derived_required_bytes=self.relations.derived.working_bytes(),
            derived_peak_required_bytes=self.relations.derived.peak_working_bytes(),
            derived_retained_bytes=self.relations.derived.retained_bytes(),
            derived_owned_bytes=self.relations.derived.owned_bytes(),
            derived_backend_overlap_bytes=self.relations.derived.required_backend_overlap(),
            derived_peak_required_views=self.relations.derived.peak_required_views(),
            derived_metadata_bytes=self.relations.derived.metadata_bytes());
        Ok(table)
    }
}

/// Existing primitive Feed contract supplies lazy, borrowed graph dependencies.
/// OnceCell slots pin returned handles for this primitive invocation without
/// copying cells; no borrowed provider view survives a rule's keep boundary.
pub(crate) struct GraphInputs<'a, 'f> {
    execution: &'a mut FileDagExecution,
    inputs: &'a [NodeInput],
    schemas: &'a [Vec<CellKind>],
    held: &'a [OnceCell<crate::application::file_dag::relations::PublishedInput>],
    read: &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    executor: &'a dyn PrimitiveExecutor,
}
impl<'a> Feed<'a> for GraphInputs<'a, '_> {
    fn relation_owner(&self) -> Option<std::rc::Rc<super::relations::RelationOwner>> {
        Some(std::rc::Rc::clone(&self.execution.relations))
    }
    fn origin(&self, which: InputPlace) -> Result<super::feed::InputOrigin, OperatingError> {
        match self.inputs.get(which.0) {
            Some(NodeInput::Relation(relation)) => {
                Ok(super::feed::InputOrigin::Relation(*relation))
            }
            Some(NodeInput::Node(_)) => Ok(super::feed::InputOrigin::Internal),
            None => Err(OperatingError::MissingInput {
                position: which.0 + 1,
            }),
        }
    }
    fn publication(
        &self,
        which: InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        if which.0 >= self.inputs.len() {
            return Err(OperatingError::MissingInput {
                position: which.0 + 1,
            });
        }
        Ok(self
            .held
            .get(which.0)
            .and_then(OnceCell::get)
            .map(Clone::clone))
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.schemas
            .get(which.0)
            .map(Vec::as_slice)
            .ok_or(OperatingError::MissingInput {
                position: which.0 + 1,
            })
    }
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::AtHand
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'a: 't,
    {
        match self
            .inputs
            .get(which.0)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: which.0 + 1,
            })? {
            NodeInput::Relation(relation) => {
                let raw = (self.read)(relation)?;
                let slot = &self.held[which.0];
                if slot.get().is_none() {
                    let result = self.execution.relations.source(relation, &raw)?;
                    let _installed = slot.set(result);
                }
                Ok(raw)
            }
            NodeInput::Node(node) => {
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table =
                        self.execution
                            .demand_result(node, self.read, self.executor, context)?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                self.execution.relations.resolve(table.as_ref(), self.read)
            }
        }
    }
    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let input = self
            .inputs
            .get(which.0)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: which.0 + 1,
            })?;
        let mut row = Vec::new();
        match input {
            NodeInput::Relation(relation) => {
                let raw = (self.read)(relation)?;
                let record = self.execution.relations.source(relation, &raw)?;
                let rows = self.execution.relations.normalize(&record, raw)?;
                scan(&rows, &mut row, context, emit)
            }
            NodeInput::Node(node) => {
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table =
                        self.execution
                            .demand_result(node, self.read, self.executor, context)?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                scan(
                    &self
                        .execution
                        .relations
                        .resolve(table.as_ref(), self.read)?,
                    &mut row,
                    context,
                    emit,
                )
            }
        }
    }
}

// Interior mutability is restricted to this application scheduling boundary.
// A graph demand finishes before the opaque consumer resumes, so callbacks do
// not retain a mutable borrow across arbitrary rule code.
impl DagExecution for std::cell::RefCell<FileDagExecution> {
    fn relation_owner(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.borrow().relations)
    }
    fn begin_rule(&self, rule: usize) -> Result<(), OperatingError> {
        self.borrow_mut().begin_rule(rule)
    }
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId> {
        self.borrow().owned(rule, step)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &dyn PrimitiveExecutor,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        self.borrow_mut()
            .demand_result(node, read, executor, context)
    }
}
#[cfg(test)]
impl DagExecution for std::cell::RefCell<&mut FileDagExecution> {
    fn relation_owner(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.borrow().relations)
    }
    fn begin_rule(&self, rule: usize) -> Result<(), OperatingError> {
        self.borrow_mut().begin_rule(rule)
    }
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId> {
        self.borrow().owned(rule, step)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &dyn PrimitiveExecutor,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        self.borrow_mut()
            .demand_result(node, read, executor, context)
    }
}

#[cfg(test)]
#[path = "service/required_test.rs"]
mod required_test;

impl GraphInputs<'_, '_> {
    pub(super) fn publication(
        &mut self,
        which: usize,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        match self
            .inputs
            .get(which)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: which + 1,
            })? {
            NodeInput::Relation(relation) => self
                .execution
                .relations
                .source(relation, &(self.read)(relation)?),
            NodeInput::Node(node) => {
                let slot = &self.held[which];
                if slot.get().is_none() {
                    let value =
                        self.execution
                            .demand_result(node, self.read, self.executor, context)?;
                    let _installed = slot.set(value);
                }
                Ok(slot.get().ok_or(OperatingError::NotRun)?.clone())
            }
        }
    }
    pub(super) fn inspect_publication(
        &self,
        result: &super::relations::RelationResult,
        owner: &super::relations::RelationOwner,
    ) -> Result<(), OperatingError> {
        let rows = owner.resolve(result, self.read)?;
        let _shape = owner.inspect(
            result,
            crate::domain::operator::contract::views::BorrowedRows::from(&rows),
        )?;
        Ok(())
    }
    pub(super) fn produce(
        &self,
        program: &Arc<crate::domain::stored_selection::types::Selector>,
        owner: &super::relations::RelationOwner,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<super::relations::ProducedSelection>, OperatingError> {
        owner.produce_selection(program, self.read, context)
    }
}

impl FileDagExecution {
    /// Invalidate all borrowed-population descriptors before provider mutation.
    pub(crate) fn release_population(&mut self) {
        self.ready.fill(None);
        self.required.fill(None);
        self.retained_bytes = 0;
        self.required_bytes = 0;
        self.overlap_bytes = 0;
        self.relations.release_population();
    }
    #[cfg(test)]
    pub(crate) fn demand<'f>(
        &mut self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Arc<Table>, OperatingError> {
        let result = self.demand_result(
            node,
            read,
            &super::execution::CommonOperatorEngine::fixture(),
            context,
        )?;
        let rows = self.relations.resolve(result.as_ref(), read)?;
        let mut table = Table::default();
        crate::domain::table_model::service::materialize(&rows, &mut table, &mut Vec::new());
        Ok(Arc::new(table))
    }
}

#[cfg(test)]
#[path = "service/representation_test.rs"]
mod representation_test;

#[cfg(test)]
#[path = "service/executor_test.rs"]
mod executor_test;
