//! Which rules and producer frontiers a changed selection reaches through named observations.
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::prepared_execution::types::ReplayClosure;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::stores::types::{MadeAgain, Reuse};
use std::collections::VecDeque;

/// Rules reached so far and the ones whose reach is still to be followed.
/// Whether the reach of a changed selection has arrived at a rule.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Untouched,
    Reached,
}
struct Reach {
    needed: Vec<Visit>,
    queue: VecDeque<RuleIndex>,
}
impl Reach {
    fn mark(&mut self, rule: RuleIndex) -> Result<(), OperatingError> {
        let mark = self.needed.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
        if *mark == Visit::Untouched {
            *mark = Visit::Reached;
            self.queue.push_back(rule);
        }
        Ok(())
    }
}
impl PreparedProgram {
    pub(crate) fn replay_closure(
        &self,
        again: Reuse,
        first: &[ryo_values::domain::stored_facts::types::Bucket],
    ) -> Result<ReplayClosure, OperatingError> {
        let Some(named) = &self.observations else {
            return Ok(ReplayClosure {
                reuse: again,
                buckets: vec![],
            });
        };
        let Reuse::Cross(MadeAgain::These(selected)) = again else {
            return Ok(ReplayClosure {
                reuse: again,
                buckets: vec![],
            });
        };
        let mut reach = Reach {
            needed: vec![Visit::Untouched; self.rules.len()],
            queue: VecDeque::new(),
        };
        for selected in selected {
            reach.mark(RuleIndex(selected.0 as usize))?;
        }
        while let Some(rule) = reach.queue.pop_front() {
            for output in named.consumers.get(rule.0).ok_or(OperatingError::NotRun)? {
                for &consumer in output {
                    reach.mark(consumer)?;
                }
            }
            self.follow_producers(named, rule, &mut reach)?;
        }
        let mut buckets = std::collections::BTreeSet::new();
        for root in named.frontiers.iter() {
            let consumers = named
                .consumers
                .get(root.producer.0)
                .and_then(|outputs| outputs.get(root.output.0 as usize))
                .ok_or(OperatingError::NotRun)?;
            if consumers
                .iter()
                .any(|consumer| reach.needed[consumer.0] == Visit::Reached)
            {
                buckets.insert(ryo_values::domain::stored_facts::service::frontier_bucket(
                    *first.get(root.producer.0).ok_or(OperatingError::NotRun)?,
                    root.frontier,
                ));
            }
        }
        let selected = reach
            .needed
            .into_iter()
            .enumerate()
            .filter(|(_, needed)| *needed == Visit::Reached)
            .map(|(rule, _)| {
                u32::try_from(rule)
                    .map(ryo_values::domain::row_texts::types::PlacedRule)
                    .map_err(|_| OperatingError::NotRun)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ReplayClosure {
            reuse: Reuse::Cross(MadeAgain::These(selected)),
            buckets: buckets.into_iter().collect(),
        })
    }
    /// A consumer reads the cross-file outputs of its producers, so those producers rerun too.
    fn follow_producers(
        &self,
        named: &ObservationMetadata,
        rule: RuleIndex,
        reach: &mut Reach,
    ) -> Result<(), OperatingError> {
        for binding in named.bindings.get(rule.0).ok_or(OperatingError::NotRun)? {
            for producer in &binding.producers {
                let ResolvedObservationProducer::Output { rule, output } = producer else {
                    continue;
                };
                let root = self
                    .rule_metadata(*rule)?
                    .observations
                    .get(output.0 as usize)
                    .ok_or(OperatingError::NotRun)?;
                if root.phase == StepScope::CrossFile {
                    reach.mark(*rule)?;
                }
            }
        }
        Ok(())
    }
}
