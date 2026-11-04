//! Replace pair enumeration with exact per-partition extrema, preserving source row order.
use crate::domain::contract::types::{
    ExtremumWitness, OrderedExistenceDriver, OrderedExistenceScratch, PartitionExtremum,
    PortableOperatorFault,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) fn flag(
    partition: &[ColumnIndex],
    order: ColumnIndex,
    extremum: PartitionExtremum,
    driver: &mut dyn OrderedExistenceDriver,
    scratch: &mut OrderedExistenceScratch,
) -> Result<(), PortableOperatorFault> {
    scratch.groups.clear();
    scratch.key.clear();
    let result = (|| {
        let rows = driver.len();
        for row in 0..rows {
            scratch.key.clear();
            for column in partition {
                scratch.key.push(
                    driver
                        .key_cell(row, *column)
                        .map_err(|_| PortableOperatorFault::Interrupted)?,
                );
            }
            let value = driver
                .number(row, order)
                .map_err(|_| PortableOperatorFault::Interrupted)?;
            if let Some(current) = scratch.groups.get_mut(scratch.key.as_slice()) {
                let replace = match extremum {
                    PartitionExtremum::Maximum => value > current.value,
                    PartitionExtremum::Minimum => value < current.value,
                };
                if replace {
                    *current = ExtremumWitness { value, row };
                }
            } else {
                scratch
                    .groups
                    .insert(scratch.key.clone(), ExtremumWitness { value, row });
            }
        }
        for row in 0..rows {
            scratch.key.clear();
            for column in partition {
                scratch.key.push(
                    driver
                        .key_cell(row, *column)
                        .map_err(|_| PortableOperatorFault::Interrupted)?,
                );
            }
            let value = driver
                .number(row, order)
                .map_err(|_| PortableOperatorFault::Interrupted)?;
            let bound = scratch
                .groups
                .get(scratch.key.as_slice())
                .ok_or(PortableOperatorFault::ShapeMismatch)?;
            let dominated = match extremum {
                PartitionExtremum::Maximum => value < bound.value,
                PartitionExtremum::Minimum => value > bound.value,
            };
            if dominated {
                driver
                    .emit_dominated(row, bound.row)
                    .map_err(|_| PortableOperatorFault::Interrupted)?;
            }
        }
        Ok(())
    })();
    // The caller owns capacity and lifetime. No successful/error invocation leaves semantic state.
    scratch.groups.clear();
    scratch.key.clear();
    result
}

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;
