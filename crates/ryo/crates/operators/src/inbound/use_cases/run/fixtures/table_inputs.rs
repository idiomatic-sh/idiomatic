//! What `TableInputs` lends a run: its tables' rows, counted per read, what it holds after the files, and each node's
//! activity, a node of its dormant set dormant in its layout.
use super::TableInputs;
use super::demands::Demand;
use crate::domain::physical_plan::types::activity::value::{Activity, NodeLayout};
use crate::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use crate::domain::physical_plan::types::candidates::value::Candidates;
use crate::domain::physical_plan::types::fact_rows::value::FactRows;
use crate::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use crate::domain::physical_plan::types::held_rows::value::HeldRows;
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId};
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::HostedNumber;

impl PlanInputs for TableInputs {
    fn facts(&self, relation: Relation) -> FactRows<'_> {
        self.after
            .demands
            .asked(relation, self.held.iter().map(|(held, _)| *held));
        match self
            .held
            .iter()
            .zip(&self.reads)
            .find(|((held, _), _)| *held == relation)
        {
            Some(((_, columns), count)) => {
                count.set(count.get() + 1);
                FactRows::Lent(ColumnsView {
                    kinds: ryo_relations::domain::relations::fixtures::kinds(relation),
                    columns,
                })
            }
            None => FactRows::Refused,
        }
    }
    fn roots(&self, _root: PlanNodeId) -> Candidates<'_> {
        Candidates(&self.roots)
    }
    fn frontier(&self, _place: FrontierPlace) -> HeldRows<'_> {
        self.after.held(Demand::Frontier)
    }
    fn observation(&self, _place: BoundObservationPlace) -> HeldRows<'_> {
        self.after.held(Demand::Observation)
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.after.hosted(relation)
    }
    fn activity(&self, node: PlanNodeId) -> Activity<'_> {
        match self.dormant.iter().find(|(dormant, _)| *dormant == node) {
            Some((_, kinds)) => Activity::Dormant(NodeLayout(kinds)),
            None => Activity::Active,
        }
    }
}
