//! The native key order each region with native positions reads.
use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;
use std::sync::Arc;

/// One entry per region, in region order: its raw key order, or none without native positions.
pub(in crate::application::file_dag::program::preparation) fn prepare(
    program: &ExecutableProgram,
    derived: &crate::application::file_dag::derived::owner::types::DerivedAccess,
) -> Result<Box<[Option<Arc<crate::domain::derived_views::types::contract::value::DerivedContract>>]>, OperatingError>
{
    Ok(program
        .regions()
        .iter()
        .map(|region| {
            if crate::domain::executable_program::service::native_positions(&region.route()).is_none() {
                return Ok(None);
            }
            let stride = match &region.instructions()[0].operation() {
                Operation::Project {
                    columns,
                    distinct: RepeatedRows::Dropped,
                    ..
                } => Some(columns.len().max(1)),
                Operation::Union { .. } => crate::domain::executable_program::service::native_positions(&region.route())
                    .map(|shape| shape.columns.len().max(1)),
                Operation::Antijoin { right_on, .. } | Operation::Join { right_on, .. } => {
                    Some(right_on.len())
                }
                _ => None,
            };
            stride
                .map(|stride| {
                    crate::application::file_dag::derived::prepare::view(
                        derived.clone(),
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
