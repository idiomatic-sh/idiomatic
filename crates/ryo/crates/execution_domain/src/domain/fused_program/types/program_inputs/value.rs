//! The input lists of a logical program's nodes, one per node, held by the caller for as long as the program borrows them.

use ryo_operators::domain::physical_plan::types::PlanInput;

/// The input lists of a logical program's nodes, in node order.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::fused_program::types::program_inputs::value::ProgramInputs;
/// use ryo_execution_domain::domain::fused_program::service::logical;
/// let mut inputs = ProgramInputs::default();
/// assert!(logical(&[], &mut inputs).is_ok());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramInputs {
    /// Each node's inputs, in the order its kind reads them.
    pub(crate) lists: Vec<Vec<PlanInput>>,
}
