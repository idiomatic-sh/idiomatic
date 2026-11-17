//! The operators port with every node active: a decorator over another adapter that lends each run inputs answering every
//! node `Active`, so a file runs as it would if it held every trigger. The gated run's answers are compared against it.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::activity::value::Activity;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::candidates::value::Candidates;
use ryo_operators::domain::physical_plan::types::fact_rows::value::FactRows;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::HostedNumber;
use std::sync::Arc;

/// The operators of `inner`, every node of every run active.
#[derive(Debug)]
pub struct AllActiveOperators {
    /// The adapter that plans and runs.
    pub inner: Arc<dyn OperatorsPort>,
}

impl OperatorsPort for AllActiveOperators {
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
        self.inner
            .run(plan, node, buffers, &EveryNodeActive(inputs), resources)
    }
}

/// The engine's inputs of one run, every node answered active.
struct EveryNodeActive<'i>(&'i dyn PlanInputs);

impl PlanInputs for EveryNodeActive<'_> {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        self.0.facts(relation)
    }
    fn roots(&self, root: PlanNodeId) -> Candidates<'_> {
        self.0.roots(root)
    }
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_> {
        self.0.frontier(place)
    }
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_> {
        self.0.observation(place)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.0.hosted(relation)
    }
    fn activity(&self, _node: PlanNodeId) -> Activity<'_> {
        Activity::Active
    }
}
