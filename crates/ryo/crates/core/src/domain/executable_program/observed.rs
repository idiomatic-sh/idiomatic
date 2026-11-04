//! Named distinct-set combinations are ordinary prepared program instructions.
use super::types::*;
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::GraphObservations,
    operating::{errors::OperatingError, types::Supply},
};
use std::sync::Arc;

pub(super) fn compile_unions(
    regions: &mut Vec<Region>,
    named: Option<&GraphObservations>,
    partitions: &dyn ProducerPartitions,
    first_logical: usize,
) -> Result<Vec<Result<RegionId, OperatingError>>, OperatingError> {
    let Some(named) = named else {
        return Ok(Vec::new());
    };
    let mut outputs = Vec::with_capacity(named.slots.len());
    let mut next_logical = first_logical;
    for (slot, declaration) in named.slots.iter().enumerate() {
        let compiled = (|| {
            let rule = RuleIndex(declaration.rule);
            let binding = named
                .bindings
                .get(declaration.rule)
                .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                .ok_or(OperatingError::NotRun)?;
            let partition = partitions.observation(rule, declaration.input)?;
            let contract = binding.union.as_ref().map_err(Clone::clone)?;
            let used = binding.input.producers.len();
            if used == 0 {
                return Err(OperatingError::ObservationIncomplete {
                    reason: "named input has no admitted producer population".into(),
                });
            }
            let mut level: Vec<_> = (0..used)
                .map(|member| InputSource::ObservationMember { slot, member })
                .collect();
            if level.len() == 1 {
                level.push(level[0]);
            }
            while level.len() > 1 {
                let mut next = Vec::with_capacity(level.len().div_ceil(2));
                for pair in level.chunks(2) {
                    if pair.len() == 1 {
                        next.push(pair[0]);
                        continue;
                    }
                    let id = RegionId(regions.len());
                    let logical = LogicalInstruction(next_logical);
                    next_logical += 1;
                    for source in pair {
                        if let InputSource::Region(child) = source {
                            regions[child.0].consumers += 1;
                        }
                    }
                    regions.push(Region {
                        id,
                        producer: rule,
                        needs: Ok(Box::new([])),
                        phase: binding.input.schema.scope,
                        inputs: pair.to_vec().into_boxed_slice(),
                        consumers: 0,
                        dependencies: Box::new([]),
                        ordered_existence: None,
                        column: None,
                        publishes: true,
                        instructions: vec![Instruction {
                            logical,
                            producer: rule,
                            operation: contract.identity.clone(),
                            contract: Arc::clone(contract),
                            input_delivery: vec![Supply::AtHand, Supply::AtHand].into_boxed_slice(),
                            inputs: vec![
                                OperandSource::Boundary(InputId(0)),
                                OperandSource::Boundary(InputId(1)),
                            ]
                            .into_boxed_slice(),
                            streaming_demand: super::service::demand(
                                contract.streaming,
                                2,
                                HeldJoinOrder::LeftWithEmptyGate,
                            ),
                            collected_demand: super::service::demand(
                                contract.collected,
                                2,
                                HeldJoinOrder::LeftWithEmptyGate,
                            ),
                            partition,
                        }]
                        .into_boxed_slice(),
                    });
                    next.push(InputSource::Region(id));
                }
                level = next;
            }
            let current = level[0];
            if let InputSource::Region(root) = current {
                regions[root.0].consumers += 1;
            }
            match current {
                InputSource::Region(id) => Ok(id),
                _ => Err(OperatingError::NotRun),
            }
        })();
        outputs.push(compiled);
    }
    Ok(outputs)
}
