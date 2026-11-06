//! The native key order each region with native positions reads.
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::derived::interface::DerivedPort;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;
use std::sync::Arc;

/// One entry per region, in region order: its raw key order, or none without native positions.
pub(in crate::application::file_dag::program::preparation) fn prepare(
    program: &ExecutableProgram,
    derived: &Arc<dyn DerivedPort>,
) -> Result<Box<[Option<Arc<crate::domain::derived_views::types::DerivedContract>>]>, OperatingError>
{
    Ok(program
        .regions()
        .iter()
        .map(|region| {
            if region.route.native_positions().is_none() {
                return Ok(None);
            }
            let stride = match &region.instructions[0].operation {
                Operation::Project {
                    columns,
                    distinct: RepeatedRows::Dropped,
                    ..
                } => Some(columns.len().max(1)),
                Operation::Union { .. } => region
                    .route
                    .native_positions()
                    .map(|shape| shape.columns.len().max(1)),
                Operation::Antijoin { right_on, .. } | Operation::Join { right_on, .. } => {
                    Some(right_on.len())
                }
                _ => None,
            };
            stride
                .map(|stride| {
                    crate::application::file_dag::derived::prepare::view(
                        Arc::clone(derived),
                        &crate::domain::derived_views::types::ViewRequest::RawKeyOrder { stride },
                        &[],
                    )
                    .map(Arc::new)
                })
                .transpose()
        })
        .collect::<Result<Vec<_>, OperatingError>>()?
        .into_boxed_slice())
}
