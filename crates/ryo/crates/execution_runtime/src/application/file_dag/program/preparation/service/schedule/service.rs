//! Rule order, observation batches and the demand a request places on its rules.
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::preparation::types::RuleTotal;
use crate::application::file_dag::program::prepared_execution::types::Demand;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::observation_schedule::types::{
    ResolvedObservationProducer, RuleSchedule,
};

impl PreparedProgram {
    pub(crate) fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]> {
        match &self.schedule {
            RuleSchedule::Independent => None,
            RuleSchedule::Dependencies { batches, .. } => Some(batches),
        }
    }
    pub(crate) fn rule_count(&self) -> RuleTotal {
        RuleTotal(self.rules.len())
    }
    pub(crate) fn rule_order(
        &self,
    ) -> crate::application::file_dag::program::prepared_execution::types::RuleOrder<'_> {
        use crate::application::file_dag::program::prepared_execution::types::RuleOrder;
        match &self.schedule {
            RuleSchedule::Independent => RuleOrder::Independent(0..self.rules.len()),
            RuleSchedule::Dependencies { order, .. } => RuleOrder::Dependencies(order.iter()),
        }
    }
    pub(crate) fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Vec<Demand> {
        let mut demanded: Vec<_> = (0..self.rules.len())
            .map(|place| governs(RuleIndex(place)))
            .collect();
        if let Some(observations) = &self.observations {
            for consumer in self.rule_order().collect::<Vec<_>>().into_iter().rev() {
                if demanded[consumer.0] == Demand::Undemanded {
                    continue;
                }
                for binding in &observations.bindings[consumer.0] {
                    for producer in &binding.producers {
                        if let ResolvedObservationProducer::Output { rule, .. } = producer {
                            demanded[rule.0] = Demand::Demanded;
                        }
                    }
                }
            }
        }
        demanded
    }
}
