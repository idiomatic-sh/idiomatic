//! The support of the `lifetimes` cases: the operators port a case watches, the engine's own operators, each run logged with
//! whether each watched node held rows before and after it, each relation lent, and a node made to fault; and the request
//! prepared over it.
use super::super::fixtures::{engine, request};
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::candidates::value::Candidates;
use ryo_operators::domain::physical_plan::types::fact_rows::value::FactRows;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{NodeRows, PlanInputs, PlanNodeId, Resources};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::query_plans::types::Plan;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

/// What the watched operators saw, in order: each node the engine ran, each relation a run was lent, whether each watched node
/// held rows before and after each run, and each node made to fail.
#[derive(Debug, Default)]
pub(super) struct Watch {
    /// The nodes whose rows each run reports, before and after it.
    pub(super) nodes: Mutex<Vec<PlanNodeId>>,
    /// The node every run of which faults, when one is set.
    pub(super) failing: Mutex<Option<PlanNodeId>>,
    /// What was seen, in order.
    pub(super) log: Mutex<Vec<String>>,
}

impl Watch {
    /// Adds what was seen.
    fn push(&self, event: String) {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }

    /// Everything seen so far, emptied.
    pub(super) fn taken(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Watches `nodes` from now on.
    pub(super) fn watch(&self, nodes: &[PlanNodeId]) {
        *self.nodes.lock().unwrap_or_else(PoisonError::into_inner) = nodes.to_vec();
    }

    /// Whether each watched node holds rows in `buffers`, as text.
    fn held(&self, buffers: &FileBuffers) -> String {
        let nodes = self.nodes.lock().unwrap_or_else(PoisonError::into_inner);
        let mut held = String::new();
        for node in nodes.iter() {
            let state = match buffers.rows(*node) {
                NodeRows::Materialized(_) => "held",
                NodeRows::Absent => "absent",
            };
            let _ = write!(held, " {}:{state}", node.0);
        }
        held
    }
}

/// The engine's own operators, each run seen by `watch`.
#[derive(Debug)]
pub(super) struct Watched {
    /// The operators that plan and run.
    pub(super) inner: Arc<dyn OperatorsPort>,
    /// What sees each run.
    pub(super) watch: Arc<Watch>,
}

/// The inputs of one run, each relation lent logged.
struct Lending<'i> {
    /// The run's own inputs.
    inner: &'i dyn PlanInputs,
    /// Where each lend is logged.
    watch: &'i Watch,
}

impl PlanInputs for Lending<'_> {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        self.watch.push(format!("lend {relation:?}"));
        self.inner.facts(relation)
    }
    fn roots(&self, root: PlanNodeId) -> Candidates<'_> {
        self.inner.roots(root)
    }
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_> {
        self.inner.frontier(place)
    }
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_> {
        self.inner.observation(place)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.inner.hosted(relation)
    }
}

impl OperatorsPort for Watched {
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault> {
        self.inner.plan(program)
    }
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault> {
        self.watch
            .push(format!("run {} before{}", node.0, self.watch.held(buffers)));
        let failing = *self
            .watch
            .failing
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if failing == Some(node) {
            self.watch.push(format!("fail {}", node.0));
            return Err(PortableOperatorFault::WrongOperation);
        }
        let lending = Lending {
            inner: inputs,
            watch: &self.watch,
        };
        let ran = self.inner.run(plan, node, buffers, &lending, resources);
        self.watch
            .push(format!("ran {} after{}", node.0, self.watch.held(buffers)));
        ran
    }
}

/// The request of `plans`, one rule each with no binding, prepared over the engine's own operators, each run seen by the
/// returned watch.
pub(super) fn watched(plans: Vec<Plan>) -> (RequestExecution, Arc<Watch>) {
    let watch = Arc::new(Watch::default());
    let operators: Arc<dyn OperatorsPort> = Arc::new(Watched {
        inner: engine(),
        watch: Arc::clone(&watch),
    });
    (request(plans, operators), watch)
}
