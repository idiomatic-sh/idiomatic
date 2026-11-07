//! Named distinct-set combinations are ordinary prepared program instructions.
use super::types::demand_program::value::DemandProgram;
use super::types::instruction::value::Instruction;
use super::types::region::value::Region;
use super::types::*;
use crate::domain::{
    file_program::types::GraphObservations,
    operating::errors::OperatingError,
};
use std::sync::Arc;

pub(super) fn compile_unions(
    regions: &mut Vec<Region>,
    named: Option<&GraphObservations>,
    partitions: &dyn ProducerPartitions,
    LogicalInstruction(first_logical): LogicalInstruction,
) -> Result<Vec<Result<RegionId, OperatingError>>, OperatingError> {
    let Some(named) = named else {
        return Ok(Vec::new());
    };
    let mut outputs = Vec::with_capacity(named.slots.len());
    let mut next_logical = first_logical;
    for (slot, declaration) in named.slots.iter().enumerate() {
        let compiled = (|| {
            let rule = declaration.rule;
            let binding = named
                .bindings
                .get(declaration.rule.0)
                .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                .ok_or(OperatingError::NotRun)?;
            let partition = partitions.observation(rule, declaration.input)?;
            let contract = binding.union.as_ref().map_err(Clone::clone)?;
            let used = binding.input.producers.len();
            // Empty is authoritative only after the actual declaration and its
            // schema/phase have passed ProducerPartitions validation above.
            // It still runs the ordinary typed union and publication path.
            let mut level: Vec<_> = if used == 0 {
                vec![InputSource::EmptyObservation(ObservationPlace(slot)); 2]
            } else {
                (0..used)
                    .map(|member| InputSource::ObservationMember {
                        slot: ObservationPlace(slot),
                        member: ProducerMember(member),
                    })
                    .collect()
            };
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
                            regions[child.0].add_consumer();
                        }
                    }
                    let instruction = Instruction::new(
                        logical,
                        rule,
                        contract.identity.clone(),
                        Arc::clone(contract),
                        vec![
                            OperandSource::Boundary(InputId(0)),
                            OperandSource::Boundary(InputId(1)),
                        ]
                        .into_boxed_slice(),
                        DemandProgram::compiled(
                            contract.semantics,
                            ryo_values::domain::plan_parts::types::InputCount(2),
                        ),
                        partition,
                    );
                    regions.push(Region::new(
                        id,
                        rule,
                        Ok(Box::new([])),
                        binding.input.schema.scope,
                        vec![instruction].into_boxed_slice(),
                        pair.to_vec().into_boxed_slice(),
                        ReaderMultiplicity(0),
                    ));
                    next.push(InputSource::Region(id));
                }
                level = next;
            }
            let current = level[0];
            if let InputSource::Region(root) = current {
                regions[root.0].add_consumer();
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
