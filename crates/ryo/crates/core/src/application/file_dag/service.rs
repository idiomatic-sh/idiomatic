//! Demand a unique dependency schedule; primitive semantics stay in domain.
use super::types::FileDagExecution;
use crate::domain::file_program::types::{DagExecution, NodeId, NodeInput, RequestGraph};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
use crate::domain::replay::service::scan;
use crate::domain::replay::types::Collect;
use crate::domain::table_model::service::reset;
use crate::domain::table_model::types::{RowCount, Rows, Table};
use ryo_language::domain::relations::types::{CellKind, Relation, Value};
use std::cell::OnceCell;
use std::sync::Arc;

/// Retained result buffers only; injected typed indexes keep their existing allowance.
const RESULT_LIMIT: usize = 6 * 1024 * 1024;
fn bytes(table: &Table) -> usize {
    table
        .cells
        .capacity()
        .saturating_mul(size_of::<Value>())
        .saturating_add(table.kinds.capacity().saturating_mul(size_of::<CellKind>()))
        .saturating_add(size_of::<Table>() + 2 * size_of::<usize>())
}
impl FileDagExecution {
    pub(crate) fn new(
        graph: Arc<RequestGraph>,
        active: impl Fn(usize) -> bool,
        complete: &[Relation],
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
            graph,
            active,
            remaining,
            ready: vec![None; count],
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
        if let Some(table) = self.ready[node.0].take() {
            self.retained_bytes = self.retained_bytes.saturating_sub(bytes(&table));
            self.releases += 1;
        }
        // A lazy primitive may never request one branch. Cancel that branch's
        // outgoing consumer edges when it becomes unreachable, not at file end.
        if self.executions[node.0] == 0 {
            let graph = Arc::clone(&self.graph);
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
        let graph = Arc::clone(&self.graph);
        for (_, root) in &graph.boundaries[rule] {
            self.consumed(*root);
        }
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
    fn demand<'f>(
        &mut self,
        id: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Arc<Table>, OperatingError> {
        self.demands += 1;
        if let Some(table) = self.ready.get(id.0).and_then(Option::as_ref) {
            self.hits += 1;
            return Ok(Arc::clone(table));
        }
        if self.visiting.get(id.0).copied().unwrap_or(true) {
            return Err(OperatingError::NotRun);
        }
        self.visiting[id.0] = true;
        let graph = Arc::clone(&self.graph);
        let node = &graph.nodes[id.0];
        if crate::domain::features::service::kind_of(&node.operation) != node.contract.kind {
            return Err(OperatingError::WrongOperation);
        }
        let held = (0..node.inputs.len())
            .map(|_| OnceCell::new())
            .collect::<Vec<_>>();
        let mut feed = GraphInputs {
            execution: self,
            graph: &graph,
            inputs: &node.inputs,
            held: &held,
            read,
        };
        let mut table = Table::default();
        reset(&mut table, &node.kinds);
        let mut room = Room::default();
        let outcome = super::execution::CommonOperatorEngine.stream(
            &node.contract,
            &mut feed,
            context,
            &mut room,
            &mut Collect(&mut table),
        );
        drop(feed);
        drop(held);
        self.visiting[id.0] = false;
        outcome?;
        self.executions[id.0] += 1;
        // Dependency edges are consumed only after a successful primitive result.
        // Failure unwinds this generation without publishing a partial result.
        for input in &node.inputs {
            if let NodeInput::Node(child) = input {
                self.consumed(*child);
            }
        }
        let table = Arc::new(table);
        let size = bytes(&table);
        if self.remaining[id.0] > 0 && self.retained_bytes.saturating_add(size) <= self.limit {
            self.retained_bytes += size;
            self.peak_bytes = self.peak_bytes.max(self.retained_bytes);
            self.ready[id.0] = Some(Arc::clone(&table));
        } else {
            // Explicit budget fallback: later demand may recompute total pure work.
            self.unretained += 1;
        }
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "dag_node",
            node = id.0, executions = self.executions[id.0], cells = table.cells.len(),
            retained_bytes = self.retained_bytes, peak_bytes = self.peak_bytes);
        Ok(table)
    }
}

/// Existing primitive Feed contract supplies lazy, borrowed graph dependencies.
/// OnceCell slots pin returned handles for this primitive invocation without
/// copying cells; no borrowed provider view survives a rule's keep boundary.
struct GraphInputs<'a, 'f> {
    execution: &'a mut FileDagExecution,
    graph: &'a RequestGraph,
    inputs: &'a [NodeInput],
    held: &'a [OnceCell<Arc<Table>>],
    read: &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
}
impl<'a> Feed<'a> for GraphInputs<'a, '_> {
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        match self.inputs.get(which.0) {
            Some(NodeInput::Relation(relation)) => {
                Ok(ryo_language::domain::relations::service::kinds(*relation))
            }
            Some(NodeInput::Node(node)) => Ok(&self.graph.nodes[node.0].kinds),
            None => Err(OperatingError::MissingInput {
                position: which.0 + 1,
            }),
        }
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
            NodeInput::Relation(relation) => (self.read)(relation),
            NodeInput::Node(node) => {
                let held = self.held;
                let slot = &held[which.0];
                if slot.get().is_none() {
                    let table = self.execution.demand(node, self.read, context)?;
                    let _installed = slot.set(table);
                }
                let table = slot.get().ok_or(OperatingError::NotRun)?;
                Ok(Rows::Computed(table.as_ref()))
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
            NodeInput::Relation(relation) => scan(&(self.read)(relation)?, &mut row, context, emit),
            NodeInput::Node(node) => {
                let table = self.execution.demand(node, self.read, context)?;
                scan(&Rows::Computed(table.as_ref()), &mut row, context, emit)
            }
        }
    }
}

// Interior mutability is restricted to this application scheduling boundary.
// A graph demand finishes before the opaque consumer resumes, so callbacks do
// not retain a mutable borrow across arbitrary rule code.
impl DagExecution for std::cell::RefCell<FileDagExecution> {
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId> {
        self.borrow().owned(rule, step)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Arc<Table>, OperatingError> {
        self.borrow_mut().demand(node, read, context)
    }
}
#[cfg(test)]
impl DagExecution for std::cell::RefCell<&mut FileDagExecution> {
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId> {
        self.borrow().owned(rule, step)
    }
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Arc<Table>, OperatingError> {
        self.borrow_mut().demand(node, read, context)
    }
}
