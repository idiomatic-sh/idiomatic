//! Demand a unique dependency schedule; primitive semantics stay in domain.
use super::executor::ScopedExecutor;
use super::interface::DagExecution;
use super::types::FileDagExecution;
use crate::application::file_dag::feed::Feed;
#[cfg(test)]
use crate::domain::file_program::types::RequestGraph;
use crate::domain::file_program::types::{NodeId, NodeInput};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Supply};
use crate::domain::replay::service::scan;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use std::cell::OnceCell;
use std::sync::Arc;

/// Canonical scope state is borrowed only for entry/commit/accounting. Neither
/// GraphInputs nor a semantic/downstream callback receives a RefMut.
pub(super) trait ScopeState {
    fn state(&self) -> std::cell::Ref<'_, FileDagExecution>;
    fn state_mut(&self) -> std::cell::RefMut<'_, FileDagExecution>;
}
impl ScopeState for std::cell::RefCell<FileDagExecution> {
    fn state(&self) -> std::cell::Ref<'_, FileDagExecution> {
        self.borrow()
    }
    fn state_mut(&self) -> std::cell::RefMut<'_, FileDagExecution> {
        self.borrow_mut()
    }
}
#[cfg(test)]
impl ScopeState for std::cell::RefCell<&mut FileDagExecution> {
    fn state(&self) -> std::cell::Ref<'_, FileDagExecution> {
        std::cell::Ref::map(self.borrow(), |state| &**state)
    }
    fn state_mut(&self) -> std::cell::RefMut<'_, FileDagExecution> {
        std::cell::RefMut::map(self.borrow_mut(), |state| &mut **state)
    }
}

enum NodeDelivery<'a> {
    Retain,
    Forward(&'a mut dyn Emit),
}
enum NodeOutput {
    Retained(super::relations::PublishedInput),
    Forwarded,
}

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
        let owner = std::rc::Rc::new(super::relations::RelationOwner::for_request(
            true,
            graph.nodes.len() + complete.len() + 1,
            crate::outbound::derived::factory::test_port(),
        ));
        Self::new_for_population(graph, active, complete, owner)
    }
    #[cfg(test)]
    pub(crate) fn new_for_population(
        graph: Arc<RequestGraph>,
        active: impl Fn(usize) -> bool,
        complete: &[Relation],
        owner: std::rc::Rc<super::relations::RelationOwner>,
    ) -> Self {
        let schedule =
            super::activation::types::ActivationCache::default().activate(&graph, active, complete);
        Self::new_for_schedule(
            &schedule,
            owner,
            super::completion::CompletionPolicy::file(
                crate::domain::coverage::types::RuleIndex(0),
                ryo_relations::domain::relations::types::FileId(0),
                &[],
            ),
        )
    }
    pub(in crate::application) fn new_for_schedule(
        schedule: &super::activation::types::ActivatedSchedule,
        owner: std::rc::Rc<super::relations::RelationOwner>,
        completion: super::completion::CompletionPolicy,
    ) -> Self {
        let graph = Arc::clone(&schedule.graph);
        let active = schedule.active.clone();
        let remaining = schedule.remaining.clone();
        let count = graph.nodes.len();
        let observations = graph
            .observations
            .as_ref()
            .map(super::observations::FileObservations::new);
        Self {
            failed: vec![None; count],
            counting: crate::domain::lowering::types::Counting::DoesNotCount,
            fed: vec![crate::domain::operator::types::RowsFed::default(); count],
            fed_rule: vec![crate::domain::operator::types::RowsFed::default(); count],
            fed_touched: Vec::new(),
            native_steps: crate::domain::operator::types::TreeSteps::AsOperators,
            kernel_strategy: crate::domain::column_kernel::types::KernelStrategy::Existing,
            kernel_counts: crate::domain::column_kernel::types::KernelCounts::default(),
            classes: Arc::new(crate::domain::shared_walk::types::SharedClasses::default()),
            class_of: Vec::new(),
            completion,
            coverage: vec![None; count],
            rooms: std::rc::Rc::new(super::invocation::rooms::InvocationRooms::new(count)),
            observations,
            relations: owner,
            graph,
            phase: ryo_values::domain::plans::types::StepScope::PerFile,
            provided: std::collections::HashMap::new(),
            provided_releases: 0,
            provided_peak_payload: 0,
            provided_allocations: Vec::new(),
            provided_observations: std::collections::HashMap::new(),
            hosted: std::collections::HashMap::new(),
            active,
            reusable: schedule.reusable.clone(),
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
    /// Native batch work is an admitted physical algorithm of this scope. It
    /// cannot choose a rule scheduler or create a result/lifetime owner.
    pub(crate) fn prepare_native(
        scope: &std::cell::RefCell<Self>,
        shared: &mut crate::domain::shared_walk::types::MergedWalk,
        walking: crate::domain::shared_walk::types::Walking<'_>,
        kernel: crate::domain::column_kernel::types::KernelStrategy,
        counting: crate::domain::lowering::types::Counting,
        executor: &ScopedExecutor<'_>,
    ) -> Result<Arc<crate::domain::shared_walk::types::SharedClasses>, OperatingError> {
        let (graph, active) = {
            let mut state = scope.borrow_mut();
            if !std::rc::Rc::ptr_eq(&state.relations, executor.owner()) {
                return Err(OperatingError::NotRead);
            }
            state.native_steps = walking.steps;
            state.kernel_strategy = kernel;
            state.counting = counting;
            (Arc::clone(&state.graph), state.active.clone())
        };
        if shared.made == crate::domain::shared_walk::types::Merging::NotYet {
            shared.merge =
                crate::domain::step_classes::service::merge(walking.plans, walking.bindings);
            shared.made = crate::domain::shared_walk::types::Merging::Done;
        }
        let outputs: Vec<NodeId> = shared
            .merge
            .classes
            .iter()
            .flat_map(|class| class.members.iter())
            .filter(|member| active.get(member.rule.0).copied().unwrap_or(false))
            .filter_map(|member| {
                graph
                    .program_nodes
                    .get(member.rule.0)
                    .and_then(|nodes| nodes.get(member.step.0))
                    .and_then(|node| node.as_ref().ok())
                    .copied()
            })
            .collect();
        // Batch admission is governed by its declared outputs, not unrelated failures.
        {
            let state = scope.borrow();
            for id in &outputs {
                if let Some(error) = state.failed.get(id.0).and_then(Option::as_ref) {
                    return Err(error.clone());
                }
            }
        }
        let declaration = super::executor::NativeClassBatch {
            graph: &graph,
            input: Relation::SyntaxTree,
            outputs,
        };
        let mut classes = std::mem::take(walking.room);
        executor.native_classes(
            &declaration,
            shared,
            crate::domain::shared_walk::types::Walking {
                room: &mut classes,
                ..walking
            },
        )?;
        let class_of = shared
            .merge
            .class_of
            .iter()
            .enumerate()
            .map(|(rule, _)| {
                let mut metadata = crate::domain::positions::types::LoweredRoom::default();
                crate::domain::shared_walk::service::install(
                    shared,
                    &classes,
                    &mut metadata,
                    crate::domain::coverage::types::RuleIndex(rule),
                );
                metadata.picked
            })
            .collect();
        // A refused callback has returned before any native result can enter
        // scope state. Commit uses a new, short borrow after semantic work.
        let classes = Arc::new(classes);
        let mut state = scope.borrow_mut();
        state.class_of = class_of;
        state.classes = Arc::clone(&classes);
        Ok(classes)
    }
    pub(super) fn consumed(&mut self, node: NodeId) {
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
        self.retire_provided(node);
        // A lazy primitive may never request one branch. Cancel that branch's
        // outgoing consumer edges when it becomes unreachable, not at file end.
        if self.executions[node.0] == 0 && !self.provided.contains_key(&node) {
            let graph = (self.graph).clone();
            for input in &graph.nodes[node.0].inputs {
                if let NodeInput::Node(child) = input {
                    self.consumed(*child);
                }
            }
        }
    }
    pub(crate) fn finish_rule(&mut self, rule: usize) {
        self.finish_observation_rule(rule);
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
    fn partial_node(
        &mut self,
        id: NodeId,
    ) -> Result<Option<super::relations::PublishedInput>, OperatingError> {
        let node = self.graph.nodes.get(id.0).ok_or(OperatingError::NotRun)?;
        match self.completion.admit(
            node.needs.as_ref().map_err(Clone::clone)?,
            &node.contract.collected_kinds,
            &self.relations,
        )? {
            super::completion::NodeAdmission::Evaluate => Ok(None),
            super::completion::NodeAdmission::Blocked { rows, coverage } => {
                let publication = rows.into_payload();
                let inputs = node.inputs.clone();
                self.coverage[id.0] = Some(coverage);
                self.completed_inputs(id, &inputs);
                self.pin_required(id, publication.clone());
                self.publish_observed_node(id, &publication);
                Ok(Some(publication))
            }
        }
    }
    fn retire_provided(&mut self, node: NodeId) {
        let retired = match self.provided.get(&node) {
            Some(
                super::types::ProvidedInput::Live { input, .. }
                | super::types::ProvidedInput::Compact { input, .. },
            ) => Some(*input),
            _ => None,
        };
        if let Some(input) = retired {
            self.provided
                .insert(node, super::types::ProvidedInput::Retired { input });
            self.provided_releases += 1;
            self.observe_provided("provided_retire");
        }
    }
    pub(super) fn retire_unused_sources(&mut self) {
        let unused = self
            .provided
            .keys()
            .copied()
            .filter(|node| self.remaining[node.0] == 0)
            .collect::<Vec<_>>();
        for node in unused {
            self.retire_provided(node);
        }
    }
    fn completed_inputs(&mut self, id: NodeId, inputs: &[NodeInput]) -> bool {
        let first_global = self.executions[id.0] == 0;
        self.executions[id.0] += 1;
        let first_required = !self.required_executed[id.0];
        self.required_executed[id.0] = true;
        for input in inputs {
            if let NodeInput::Node(child) = input {
                if first_global {
                    self.consumed(*child);
                }
                if first_required && self.required_inputs[id.0] {
                    self.consume_required(*child);
                }
            }
        }
        first_global
    }
    /// One prepared physical alternative for both streamed and retained supply.
    /// The scheduler owns acquisition and skipped-edge cancellation in either mode.
    fn prepared_strategy<'f>(
        scope: &dyn ScopeState,
        id: NodeId,
        graph: &crate::domain::file_program::types::RequestGraph,
        relations: &std::rc::Rc<super::relations::RelationOwner>,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut crate::domain::operating::types::Room,
    ) -> Result<super::ordered_existence::StrategyOutcome, OperatingError> {
        let node = graph.nodes.get(id.0).ok_or(OperatingError::NotRun)?;
        use crate::domain::ordered_existence::types::ResolvedAlternative;
        let alternative = graph.ordered_existence.get(id.0).and_then(Option::as_ref);
        if let Some(ResolvedAlternative::Declined(reason)) = alternative {
            let _ = reason;
            super::route_counts::event("ordered_existence", "declined_shared_consumer");
        }
        if let Some(ResolvedAlternative::Eligible(shape)) = alternative {
            // The request compiler matched original plan coordinates to these
            // exact producer edges. Demand only S: J/F remain unexecuted and
            // their ordinary edges are cancelled by consumed below.
            if shape.select != id || node.inputs != [NodeInput::Node(shape.join)] {
                return Err(OperatingError::WrongOperation);
            }
            let source = Self::demand_scoped(scope, shape.source, read, executor, context)?;
            let rows = relations.resolve(source.as_ref(), read)?;
            match executor.ordered_existence(
                shape,
                &source,
                crate::domain::operator::contract::views::BorrowedRows::from(&rows),
                node.contract.schema.as_ref().map_err(Clone::clone)?,
                context,
                room,
            )? {
                super::ordered_existence::StrategyOutcome::Executed(result) => {
                    super::route_counts::event("ordered_existence", "executed_skipped_join");
                    return Ok(super::ordered_existence::StrategyOutcome::Executed(result));
                }
                super::ordered_existence::StrategyOutcome::Declined(_reason) => {
                    super::route_counts::event("ordered_existence", "declined_population");
                }
            }
            // Admission declined before output. Ordinary evaluation retains
            // all malformed/unsupported representation and fault behavior.
        }
        Ok(super::ordered_existence::StrategyOutcome::Declined(
            super::ordered_existence::AdmissionDecline::NoEligiblePlan,
        ))
    }
    pub(super) fn stream_scoped<'f>(
        scope: &dyn ScopeState,
        id: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Self::invoke_node(
            scope,
            id,
            read,
            executor,
            context,
            &mut NodeDelivery::Forward(emit),
        )
        .map(|_| ())
    }
    pub(super) fn demand_scoped<'f>(
        scope: &dyn ScopeState,
        id: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::relations::PublishedInput, OperatingError> {
        match Self::invoke_node(
            scope,
            id,
            read,
            executor,
            context,
            &mut NodeDelivery::Retain,
        )? {
            NodeOutput::Retained(publication) => Ok(publication),
            NodeOutput::Forwarded => Err(OperatingError::WrongOperation),
        }
    }
    /// Delivery is downstream of the single admission/execution/commit owner.
    /// A callback refusal cannot invalidate an already completed producer.
    fn deliver_publication<'f>(
        delivery: &mut NodeDelivery<'_>,
        publication: super::relations::PublishedInput,
        relations: &super::relations::RelationOwner,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<NodeOutput, OperatingError> {
        match delivery {
            NodeDelivery::Retain => Ok(NodeOutput::Retained(publication)),
            NodeDelivery::Forward(emit) => {
                let rows = relations.resolve(publication.as_ref(), read)?;
                let mut scratch = crate::domain::table_scratch::types::OperatorScratch::default();
                let mut memo = Vec::new();
                let mut output = super::execution::output::State::take(&mut scratch, &mut memo);
                output.run(*emit, |emit| scan(&rows, &mut Vec::new(), context, emit))?;
                Ok(NodeOutput::Forwarded)
            }
        }
    }
    /// Resolve an admitted compact source outside every scheduler borrow. The
    /// descriptor and terminal outcome survive retirement; rows are never rebuilt.
    fn supplied_input(
        scope: &dyn ScopeState,
        id: NodeId,
        executor: &ScopedExecutor<'_>,
    ) -> Result<Option<super::relations::PublishedInput>, OperatingError> {
        use super::types::ProvidedInput;
        let (input, table, relations) = {
            let mut state = scope.state_mut();
            if !std::rc::Rc::ptr_eq(&state.relations, executor.owner()) {
                return Err(OperatingError::NotRead);
            }
            if let Some(error) = state.failed.get(id.0).and_then(Option::as_ref) {
                return Err(error.clone());
            }
            match state.provided.get(&id).cloned() {
                None => return Ok(None),
                Some(ProvidedInput::Live { publication, .. }) => return Ok(Some(publication)),
                Some(ProvidedInput::Refused { input, error }) => {
                    tracing::trace!(target:"ryo::diagnostic::lifecycle_memory",phase="provided_refusal",input=?input);
                    return Err(error);
                }
                Some(ProvidedInput::Retired { input }) => {
                    return Err(OperatingError::RetiredFrontier { input });
                }
                Some(ProvidedInput::Materializing { input }) => {
                    tracing::trace!(target:"ryo::diagnostic::lifecycle_memory",phase="provided_reentry",input=?input);
                    return Err(OperatingError::WrongOperation);
                }
                Some(ProvidedInput::Compact { input, table }) => {
                    state
                        .provided
                        .insert(id, ProvidedInput::Materializing { input });
                    (input, table, std::rc::Rc::clone(&state.relations))
                }
            }
        };
        let decoded = relations.compact(Arc::clone(&table));
        let mut state = scope.state_mut();
        match decoded {
            Ok(publication) => {
                if tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE)
                {
                    let words = crate::domain::table_words::service::compact_backing(&table);
                    let cells = words.0.0.capacity()
                        + crate::domain::table_words::service::compact_metadata_bytes(&table);
                    tracing::trace!(target:"ryo::diagnostic::lifecycle_memory", phase="compact_publication", input=?input, node=id.0,
                        rows=crate::domain::table_words::service::compact_rows(&table), kinds=?crate::domain::table_words::service::compact_kinds(&table),
                        packed_bytes=words.0.0.len(), metadata_bytes=crate::domain::table_words::service::compact_metadata_bytes(&table),
                        avoided_value_bytes=crate::domain::table_words::service::compact_cells(&table)*size_of::<ryo_relations::domain::relations::types::Value>());

                    state
                        .provided_allocations
                        .push((Arc::downgrade(publication.record()), cells));
                    state.observe_provided("provided_compact_acquire");
                }
                state.provided.insert(
                    id,
                    ProvidedInput::Live {
                        input,
                        publication: publication.clone(),
                    },
                );
                Ok(Some(publication))
            }
            Err(error) => {
                state.provided.insert(
                    id,
                    ProvidedInput::Refused {
                        input,
                        error: error.clone(),
                    },
                );
                Err(error)
            }
        }
    }
    pub(super) fn observe_provided(&mut self, phase: &'static str) {
        if !tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE) {
            return;
        }
        let live = self
            .provided_allocations
            .iter()
            .filter(|(allocation, _)| allocation.strong_count() > 0)
            .map(|(_, cells)| *cells)
            .sum::<usize>();
        self.provided_peak_payload = self.provided_peak_payload.max(live);
        // This counter owns compact source leases only. Decoded table memory is
        // measured at the actual materialization call, not guessed here.
        tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase, owner="provided_source_leases", source_payload_live_bytes=live, source_payload_peak_live_bytes=self.provided_peak_payload, publications=self.provided_allocations.len(), releases=self.provided_releases);
    }
    /// Sole node lifecycle: owner validation, admission, strategy, first error,
    /// dependency release and publication are independent of output delivery.
    fn invoke_node<'f>(
        scope: &dyn ScopeState,
        id: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
        delivery: &mut NodeDelivery<'_>,
    ) -> Result<NodeOutput, OperatingError> {
        let _node = super::route_counts::node(id.0);
        let blocked = {
            let mut state = scope.state_mut();
            if !std::rc::Rc::ptr_eq(&state.relations, executor.owner()) {
                return Err(OperatingError::NotRead);
            }
            if state
                .current_rule
                .is_none_or(|rule| !state.active.get(rule).copied().unwrap_or(false))
            {
                return Err(OperatingError::NotRun);
            }
            if let Some(error) = state.failed.get(id.0).and_then(Option::as_ref) {
                return Err(error.clone());
            }
            use super::types::ProvidedInput;
            match state.provided.get(&id) {
                Some(ProvidedInput::Refused { error, .. }) => return Err(error.clone()),
                Some(ProvidedInput::Retired { input }) => {
                    return Err(OperatingError::RetiredFrontier { input: *input });
                }
                Some(ProvidedInput::Materializing { .. }) => {
                    return Err(OperatingError::WrongOperation);
                }
                _ => {}
            }
            let node = state.graph.nodes.get(id.0).ok_or(OperatingError::NotRun)?;
            let supplied = state.provided.contains_key(&id);
            if node.phase != state.phase
                && !(supplied
                    && state.phase == ryo_values::domain::plans::types::StepScope::CrossFile
                    && node.phase == ryo_values::domain::plans::types::StepScope::PerFile)
            {
                return Err(OperatingError::NotRun);
            }
            if crate::domain::features::service::kind_of(&node.operation) != node.contract.kind {
                return Err(OperatingError::WrongOperation);
            }
            node.needs.as_ref().map_err(Clone::clone)?;
            let cached = matches!(state.provided.get(&id), Some(ProvidedInput::Live { .. }))
                || state.required.get(id.0).is_some_and(Option::is_some)
                || state.ready.get(id.0).is_some_and(Option::is_some);
            if cached {
                None
            } else {
                state
                    .partial_node(id)?
                    .map(|publication| (publication, std::rc::Rc::clone(&state.relations)))
            }
        };
        if let Some((publication, relations)) = blocked {
            return Self::deliver_publication(delivery, publication, &relations, read, context);
        }
        let supplied = Self::supplied_input(scope, id, executor)?;
        let (graph, relations, rooms, cached) = {
            let mut state = scope.state_mut();
            if !std::rc::Rc::ptr_eq(&state.relations, executor.owner()) {
                return Err(OperatingError::NotRead);
            }
            if let Some(error) = state.failed.get(id.0).and_then(Option::as_ref) {
                return Err(error.clone());
            }
            let node = state.graph.nodes.get(id.0).ok_or(OperatingError::NotRun)?;
            // Supplied frontiers are owned publications from an earlier phase.
            // Their typed refusal must survive; phase checks govern execution,
            // never consumption of a declared supplied input.
            if !state.provided.contains_key(&id) {
                if node.phase != state.phase {
                    return Err(OperatingError::NotRun);
                }
                if crate::domain::features::service::kind_of(&node.operation) != node.contract.kind
                {
                    return Err(OperatingError::WrongOperation);
                }
                node.needs.as_ref().map_err(Clone::clone)?;
            }
            state.demands += 1;
            let cached = if let Some(provided) = supplied {
                Some(provided)
            } else if let Some(table) = state.required.get(id.0).and_then(Option::as_ref).cloned() {
                state.hits += 1;
                state.publish_observed_node(id, &table);
                Some(table)
            } else if let Some(table) = state.ready.get(id.0).and_then(Option::as_ref).cloned() {
                state.hits += 1;
                let table = state.relations.required_publication(&table)?;
                state.pin_required(id, table.clone());
                state.publish_observed_node(id, &table);
                Some(table)
            } else {
                state.partial_node(id)?
            };
            if cached.is_none() {
                if state.visiting.get(id.0).copied().unwrap_or(true) {
                    return Err(OperatingError::NotRun);
                }
                state.visiting[id.0] = true;
            }
            (
                Arc::clone(&state.graph),
                std::rc::Rc::clone(&state.relations),
                std::rc::Rc::clone(&state.rooms),
                cached,
            )
        };
        if let Some(publication) = cached {
            return Self::deliver_publication(delivery, publication, &relations, read, context);
        }
        let node = &graph.nodes[id.0];
        #[cfg(feature = "diagnostic-row-progress")]
        super::route_counts::node_description(&node.operation);
        let completion = scope.state().completion.clone();
        let request_source = scope.state().hosted.get(&id).cloned();
        let mut room = match rooms.take(id) {
            Ok(room) => room,
            Err(error) => {
                scope.state_mut().visiting[id.0] = false;
                return Err(error);
            }
        };
        let mut downstream = crate::domain::operator::contract::failure::FailureLatch::default();
        let outcome = (|| {
            let room = room.room()?;
            if let super::ordered_existence::StrategyOutcome::Executed(publication) =
                Self::prepared_strategy(
                    scope, id, &graph, &relations, read, executor, context, room,
                )?
            {
                return Ok(NodeOutput::Retained(publication));
            }
            let held = (0..node.inputs.len())
                .map(|_| OnceCell::new())
                .collect::<Vec<_>>();
            let mut feed = GraphInputs {
                execution: scope,
                owner: std::rc::Rc::clone(&relations),
                node: id,
                phase: node.phase,
                region: node
                    .regions
                    .iter()
                    .find(|region| Some(region.rule) == scope.state().current_rule),
                request_source: request_source.as_ref(),
                source_kinds: node
                    .contract
                    .collected_kinds
                    .as_ref()
                    .map_or(&[], Vec::as_slice),
                inputs: &node.inputs,
                schemas: &node.contract.input_kinds,
                supply: &node.input_supply,
                held: &held,
                read,
                executor,
            };
            // Project completion filters after collection. It may not forward
            // uncertain rows before the same required policy has examined them.
            if matches!(delivery, NodeDelivery::Retain) || completion.requires_buffer() {
                return executor
                    .keep_representation(&node.contract, &node.physical, &mut feed, context, room)
                    .map(NodeOutput::Retained);
            }
            let NodeDelivery::Forward(emit) = delivery else {
                return Err(OperatingError::WrongOperation);
            };
            let mut output =
                super::execution::output::State::take(&mut room.scratch, &mut room.memo);
            let result = output.run(*emit, |emit| {
                let mut sticky = crate::domain::replay::types::Pipe(
                    |each: &mut OperatorContext<'_>, row: &[Value]| {
                        downstream.run(|| emit.row(each, row)).map_err(|()| {
                            downstream
                                .error()
                                .expect("latched downstream error")
                                .clone()
                        })
                    },
                );
                executor.stream_representation(
                    &node.contract,
                    &node.physical,
                    &mut feed,
                    context,
                    room,
                    &mut sticky,
                )
            });
            output.restore(&mut room.scratch, &mut room.memo);
            result.map(|()| NodeOutput::Forwarded)
        })();
        scope.state_mut().visiting[id.0] = false;
        if let Some(error) = downstream.take() {
            // The producer did not fail. Its edges remain available for retry.
            return Err(error);
        }
        let publication = match outcome {
            Ok(NodeOutput::Forwarded) => {
                let mut state = scope.state_mut();
                state.coverage[id.0] = Some(super::completion::CoverageState::Complete);
                state.completed_inputs(id, &node.inputs);
                return Ok(NodeOutput::Forwarded);
            }
            Ok(NodeOutput::Retained(publication)) => Ok(publication),
            Err(error) => Err(error),
        };
        let probed = usize::from(matches!(
            &node.operation,
            ryo_values::domain::plans::types::Operation::Antijoin { .. }
                | ryo_values::domain::plans::types::Operation::Difference { .. }
        ));
        let (table, coverage) = match completion.finish(
            &node.operation,
            node.needs.as_ref().map_err(Clone::clone)?,
            node.contract
                .input_kinds
                .get(probed)
                .map_or(&[], Vec::as_slice),
            publication,
            &relations,
            read,
        ) {
            super::completion::NodeCompletion::Available { rows, coverage } => {
                (rows.into_payload(), coverage)
            }
            super::completion::NodeCompletion::Failed(error) => {
                let mut state = scope.state_mut();
                state.failed[id.0] = Some(error.clone());
                state.completed_inputs(id, &node.inputs);
                return Err(error);
            }
            super::completion::NodeCompletion::Unavailable(reason) => {
                let super::completion::UnavailableReason::MissingFrontier { input } = reason.reason;
                return Err(OperatingError::FrontierUnavailable { input });
            }
        };
        let mut state = scope.state_mut();
        state.coverage[id.0] = Some(coverage);
        state.publish_observed_node(id, &table);
        if state.graph.derived_sources[id.0] {
            state.relations.bind_producer(id, table.as_ref())?;
        }
        let first_global_execution = state.completed_inputs(id, &node.inputs);
        state.pin_required(id, (table).clone());
        // Global consumers own the derived population independently of whether
        // this producer's Table fits the optional result cache. Rule-local
        // required pins remain a separate, symmetric lifetime.
        if first_global_execution && state.remaining[id.0] > 0 && state.graph.derived_sources[id.0]
        {
            state.relations.pin_derived(table.as_ref());
        }
        let size = bytes(&table);
        let retained = if state.remaining[id.0] > 0 && state.reusable[id.0] {
            state.relations.retain_publication(&table, state.limit)?
        } else {
            None
        };
        if let Some(retained) = retained {
            state.retained_bytes += size;
            state.peak_bytes = state.peak_bytes.max(state.retained_bytes);
            state.ready[id.0] = Some(retained);
            if state.required[id.0].is_some() {
                state.overlap_bytes = state.overlap_bytes.saturating_add(size);
            }
            state.observe_required();
        } else {
            // Explicit budget fallback: later demand may recompute total pure work.
            state.unretained += 1;
        }
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "dag_node",
            node = id.0, executions = state.executions[id.0], representation=?crate::domain::table_model::collections::representations::publication::PublishedRelation::kind(table.as_ref()),
            retained_bytes = state.retained_bytes, peak_bytes = state.peak_bytes,
            required_bytes=state.required_bytes, peak_required_bytes=state.peak_required_bytes,
            overlap_bytes=state.overlap_bytes, peak_owned_bytes=state.peak_owned_bytes,
            required_releases=state.required_releases,
            required_metadata_bytes=state.required_metadata_bytes(),
            derived_required_bytes=state.relations.derived.working_bytes(),
            derived_peak_required_bytes=state.relations.derived.peak_working_bytes(),
            derived_retained_bytes=state.relations.derived.retained_bytes(),
            derived_owned_bytes=state.relations.derived.owned_bytes(),
            derived_backend_overlap_bytes=state.relations.derived.required_backend_overlap(),
            derived_peak_required_views=state.relations.derived.peak_required_views(),
            derived_metadata_bytes=state.relations.derived.metadata_bytes());
        drop(state);
        Self::deliver_publication(delivery, table, &relations, read, context)
    }
}

/// Existing primitive Feed contract supplies lazy, borrowed graph dependencies.
/// OnceCell slots pin returned handles for this primitive invocation without
/// copying cells; no borrowed provider view survives a rule's keep boundary.
pub(crate) struct GraphInputs<'a, 'f> {
    owner: std::rc::Rc<super::relations::RelationOwner>,
    node: NodeId,
    phase: ryo_values::domain::plans::types::StepScope,
    region: Option<&'a crate::domain::file_program::types::PhysicalRegion>,
    request_source: Option<&'a super::relations::PublishedInput>,
    source_kinds: &'a [CellKind],
    execution: &'a dyn ScopeState,
    inputs: &'a [NodeInput],
    schemas: &'a [Vec<CellKind>],
    supply: &'a [Supply],
    held: &'a [OnceCell<crate::application::file_dag::relations::PublishedInput>],
    read: &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    executor: &'a ScopedExecutor<'a>,
}
impl<'a> Feed<'a> for GraphInputs<'a, '_> {
    fn relation_owner(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.owner)
    }
    fn origin(&self, which: InputPlace) -> Result<super::feed::InputOrigin, OperatingError> {
        match self.inputs.get(which.0) {
            Some(NodeInput::Relation(relation)) => {
                Ok(super::feed::InputOrigin::Relation(*relation))
            }
            Some(NodeInput::Node(_) | NodeInput::Observation(_)) => {
                Ok(super::feed::InputOrigin::Internal)
            }
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
        if which.0 == 0 && self.request_source.is_some() {
            return Ok(self.source_kinds);
        }
        self.schemas
            .get(which.0)
            .map(Vec::as_slice)
            .ok_or(OperatingError::MissingInput {
                position: which.0 + 1,
            })
    }
    fn streams(&self, which: InputPlace) -> Supply {
        self.supply.get(which.0).copied().unwrap_or(Supply::AtHand)
    }
    fn found(&mut self, which: InputPlace, rows: RowCount) {
        self.count_input(which, rows);
    }
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'a: 't,
    {
        if which.0 == 0
            && let Some(source) = self.request_source
        {
            return self.owner.resolve(source.as_ref(), self.read);
        }
        let rows = match self
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
                    let result = self.owner.source(relation, &raw)?;
                    let _installed = slot.set(result);
                }
                Ok(raw)
            }
            NodeInput::Observation(binding) => {
                let binding = self.execution.state().observation_slot(binding)?;
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table = FileDagExecution::observed_scoped(
                        self.execution,
                        binding,
                        self.read,
                        self.executor,
                        context,
                    )?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                self.owner.resolve(table.as_ref(), self.read)
            }
            NodeInput::Node(node) => {
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table = FileDagExecution::demand_scoped(
                        self.execution,
                        node,
                        self.read,
                        self.executor,
                        context,
                    )?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                self.owner.resolve(table.as_ref(), self.read)
            }
        }?;
        #[cfg(feature = "diagnostic-row-progress")]
        super::route_counts::input_cardinality(
            which.0,
            crate::domain::table_model::service::count_of(&rows).0,
        );
        self.count_input(which, crate::domain::table_model::service::count_of(&rows));
        Ok(rows)
    }
    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let scope = self.execution;
        let node = self.node;
        let counting = scope.state().counting;
        let mut forwarded =
            crate::domain::replay::types::Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                if counting == crate::domain::lowering::types::Counting::Counts {
                    Self::record_input(scope, node, which, RowCount(1));
                }
                emit.row(each, row)
            });
        self.stream_input(which, context, &mut forwarded)
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
    fn root(&self, rule: usize, step: usize) -> Result<NodeId, OperatingError> {
        self.borrow()
            .graph
            .program_nodes
            .get(rule)
            .and_then(|nodes| nodes.get(step))
            .cloned()
            .unwrap_or(Err(OperatingError::NotRun))
    }
    fn counts(
        &self,
        rule: usize,
    ) -> Result<Vec<crate::domain::operator::types::RowsFed>, OperatingError> {
        let state = self.borrow();
        state
            .graph
            .program_nodes
            .get(rule)
            .ok_or(OperatingError::NotRun)?
            .iter()
            .map(|node| {
                let node = node.as_ref().map_err(Clone::clone)?;
                state
                    .fed_rule
                    .get(node.0)
                    .copied()
                    .ok_or(OperatingError::NotRun)
            })
            .collect()
    }
    fn refuse_observation(
        &self,
        rule: usize,
        output: ryo_values::domain::observations::types::ObservationIndex,
        reason: ryo_values::domain::observations::types::ObservationReason,
    ) -> Result<(), OperatingError> {
        self.borrow_mut().refuse_observation(rule, output, reason)
    }
    fn observation<'f>(
        &self,
        slot: crate::domain::file_program::types::ObservationSlot,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::relations::PublishedInput, OperatingError> {
        FileDagExecution::observed_scoped(self, slot, read, executor, context)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        FileDagExecution::demand_scoped(self, node, read, executor, context)
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
    fn root(&self, rule: usize, step: usize) -> Result<NodeId, OperatingError> {
        self.borrow()
            .graph
            .program_nodes
            .get(rule)
            .and_then(|nodes| nodes.get(step))
            .cloned()
            .unwrap_or(Err(OperatingError::NotRun))
    }
    fn counts(
        &self,
        rule: usize,
    ) -> Result<Vec<crate::domain::operator::types::RowsFed>, OperatingError> {
        let state = self.borrow();
        state
            .graph
            .program_nodes
            .get(rule)
            .ok_or(OperatingError::NotRun)?
            .iter()
            .map(|node| {
                let node = node.as_ref().map_err(Clone::clone)?;
                state
                    .fed_rule
                    .get(node.0)
                    .copied()
                    .ok_or(OperatingError::NotRun)
            })
            .collect()
    }
    fn refuse_observation(
        &self,
        rule: usize,
        output: ryo_values::domain::observations::types::ObservationIndex,
        reason: ryo_values::domain::observations::types::ObservationReason,
    ) -> Result<(), OperatingError> {
        self.borrow_mut().refuse_observation(rule, output, reason)
    }
    fn observation<'f>(
        &self,
        slot: crate::domain::file_program::types::ObservationSlot,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::relations::PublishedInput, OperatingError> {
        FileDagExecution::observed_scoped(self, slot, read, executor, context)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        executor: &ScopedExecutor<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        FileDagExecution::demand_scoped(self, node, read, executor, context)
    }
}

#[cfg(test)]
#[path = "service/required_test.rs"]
mod required_test;

impl<'a, 'f> GraphInputs<'a, 'f> {
    pub(super) fn count_input(&self, which: InputPlace, rows: RowCount) {
        Self::record_input(self.execution, self.node, which, rows);
    }
    fn record_input(scope: &dyn ScopeState, node: NodeId, which: InputPlace, rows: RowCount) {
        let mut state = scope.state_mut();
        if state.counting != crate::domain::lowering::types::Counting::Counts || rows.0 == 0 {
            return;
        }
        if state
            .fed_rule
            .get(node.0)
            .is_some_and(|fed| fed.0.iter().all(|count| *count == 0))
        {
            state.fed_touched.push(node);
        }
        let FileDagExecution { fed_rule, fed, .. } = &mut *state;
        for counts in [fed_rule, fed] {
            if let Some(slot) = counts
                .get_mut(node.0)
                .and_then(|fed| fed.0.get_mut(which.0))
            {
                *slot = slot.saturating_add(u64::try_from(rows.0).unwrap_or(u64::MAX));
            }
        }
    }
    fn stream_input(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if which.0 == 0
            && let Some(source) = self.request_source
        {
            let owner = std::rc::Rc::clone(&self.owner);
            let rows = owner.resolve(source.as_ref(), self.read)?;
            return scan(&rows, &mut Vec::new(), context, emit);
        }
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
                let record = self.owner.source(relation, &raw)?;
                let rows = self.owner.normalize(&record, raw)?;
                #[cfg(feature = "diagnostic-row-progress")]
                super::route_counts::input_cardinality(
                    which.0,
                    crate::domain::table_model::service::count_of(&rows).0,
                );
                scan(&rows, &mut row, context, emit)
            }
            NodeInput::Observation(binding) => {
                let binding = self.execution.state().observation_slot(binding)?;
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table = FileDagExecution::observed_scoped(
                        self.execution,
                        binding,
                        self.read,
                        self.executor,
                        context,
                    )?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                scan(
                    &self.owner.resolve(table.as_ref(), self.read)?,
                    &mut row,
                    context,
                    emit,
                )
            }
            NodeInput::Node(node) => {
                if self.streams(which) == Supply::Streamed {
                    return FileDagExecution::stream_scoped(
                        self.execution,
                        node,
                        self.read,
                        self.executor,
                        context,
                        emit,
                    );
                }
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table = FileDagExecution::demand_scoped(
                        self.execution,
                        node,
                        self.read,
                        self.executor,
                        context,
                    )?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                let rows = self.owner.resolve(table.as_ref(), self.read)?;
                #[cfg(feature = "diagnostic-row-progress")]
                super::route_counts::input_cardinality(
                    which.0,
                    crate::domain::table_model::service::count_of(&rows).0,
                );
                scan(&rows, &mut row, context, emit)
            }
        }
    }
    pub(super) fn project_phase(&self) -> bool {
        self.phase == ryo_values::domain::plans::types::StepScope::CrossFile
    }
    pub(super) fn collect_with(
        &mut self,
        owner: &super::relations::RelationOwner,
        context: &mut OperatorContext<'_>,
        collect: impl FnOnce(
            &[Rows<'_>],
            &mut OperatorContext<'_>,
            &mut Table,
        ) -> Result<(), OperatingError>,
    ) -> Result<super::relations::PublishedInput, OperatingError> {
        let count = if self.request_source.is_some() {
            1
        } else {
            self.inputs.len()
        };
        let publications = (0..count)
            .map(|which| self.publication(which, context))
            .collect::<Result<Vec<_>, _>>()?;
        let inputs = publications
            .iter()
            .map(|publication| owner.resolve(publication.as_ref(), self.read))
            .collect::<Result<Vec<_>, _>>()?;
        for (which, rows) in inputs.iter().enumerate() {
            self.count_input(
                InputPlace(which),
                crate::domain::table_model::service::count_of(rows),
            );
        }
        let mut out = Table::default();
        collect(&inputs, context, &mut out)?;
        owner.computed(out)
    }
    pub(super) fn region_allowed(&self, nodes: &[NodeId]) -> bool {
        let state = self.execution.state();
        nodes.iter().all(|id| {
            state.graph.nodes.get(id.0).is_some_and(|node| {
                state.failed.get(id.0).is_some_and(Option::is_none)
                    && node
                        .needs
                        .as_ref()
                        .is_ok_and(|needs| state.completion.allows_region(needs))
            })
        })
    }
    pub(super) fn owner(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.owner)
    }
    pub(super) fn column_counts(&self, counts: crate::domain::column_kernel::types::KernelCounts) {
        let mut state = self.execution.state_mut();
        state.kernel_counts.eligible_chains += counts.eligible_chains;
        state.kernel_counts.covered_rows += counts.covered_rows;
        state.kernel_counts.decoded_cells += counts.decoded_cells;
        state.kernel_counts.joined_hits += counts.joined_hits;
    }
    pub(super) fn region(&self) -> Option<&crate::domain::file_program::types::PhysicalRegion> {
        self.region
    }
    pub(super) fn native_binding(
        &self,
    ) -> (
        crate::domain::operator::types::TreeSteps,
        Arc<crate::domain::shared_walk::types::SharedClasses>,
        Vec<Option<crate::domain::positions::types::ClassPlace>>,
    ) {
        let state = self.execution.state();
        (
            state.native_steps,
            Arc::clone(&state.classes),
            self.region
                .and_then(|region| state.class_of.get(region.rule))
                .cloned()
                .unwrap_or_default(),
        )
    }
    pub(super) fn column_enabled(&self) -> bool {
        self.execution.state().kernel_strategy
            == crate::domain::column_kernel::types::KernelStrategy::Column
    }
    pub(super) fn original_publication(
        &self,
        rule: usize,
        step: usize,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::relations::PublishedInput, OperatingError> {
        if self.execution.state().current_rule != Some(rule) {
            return Err(OperatingError::WrongOperation);
        }
        let node = self
            .execution
            .state()
            .graph
            .program_nodes
            .get(rule)
            .and_then(|nodes| nodes.get(step))
            .cloned()
            .unwrap_or(Err(OperatingError::NotRun))?;
        FileDagExecution::demand_scoped(self.execution, node, self.read, self.executor, context)
    }
    pub(super) fn reader(&self) -> &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError> {
        self.read
    }
    pub(super) fn publication(
        &mut self,
        which: usize,
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        if which == 0
            && let Some(source) = self.request_source
        {
            return Ok(source.clone());
        }
        match self
            .inputs
            .get(which)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: which + 1,
            })? {
            NodeInput::Relation(relation) => self.owner.source(relation, &(self.read)(relation)?),
            NodeInput::Observation(binding) => {
                let binding = self.execution.state().observation_slot(binding)?;
                let slot = &self.held[which];
                if slot.get().is_none() {
                    let value = FileDagExecution::observed_scoped(
                        self.execution,
                        binding,
                        self.read,
                        self.executor,
                        context,
                    )?;
                    let _installed = slot.set(value);
                }
                Ok(slot.get().ok_or(OperatingError::NotRun)?.clone())
            }
            NodeInput::Node(node) => {
                let slot = &self.held[which];
                if slot.get().is_none() {
                    let value = FileDagExecution::demand_scoped(
                        self.execution,
                        node,
                        self.read,
                        self.executor,
                        context,
                    )?;
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
        let owner = std::rc::Rc::clone(&self.relations);
        let engine = super::execution::CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
            owner.derived.semantic_port(),
        );
        let executor = super::executor::ScopedExecutor::new(&engine, std::rc::Rc::clone(&owner))?;
        let scope = std::cell::RefCell::new(self);
        let result =
            super::interface::DagExecution::demand(&scope, node, read, &executor, context)?;
        let rows = owner.resolve(result.as_ref(), read)?;
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

use ryo_values::domain::rows::types::{RowCount, Table};
