//! The sole public contract for portable operator semantics.
use crate::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};

/// The public capability accepts borrowed rows and caller-owned scratch.
///
/// ```
/// use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall};
/// use ryo_operators::inbound::use_cases::operators::{
///     interface::OperatorsUseCase, use_case::OperatorsUseCaseImpl,
/// };
/// use ryo_relations::domain::relations::types::{Integer, Value};
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let row = [Value::Number(Integer(7))];
/// let mut output = Vec::new();
/// assert_eq!(OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(OperatorCall::Project {
///     row: &row, columns: &[ColumnIndex(0)], target: &mut output,
/// }), Ok(OperatorAnswer::Written));
/// assert_eq!(output, row);
/// ```
///
/// Implementation layers cannot be imported by consumers.
///
/// ```compile_fail,E0603
/// use ryo_operators::domain::project::service::project;
/// let mut output = Vec::new();
/// project(&[], &[], &mut output).unwrap();
/// ```
///
/// The semantic package has no execution dependency or access to engine authority.
///
/// ```compile_fail,E0432
/// use ryo_execution::inbound::use_cases::execution::types::OperatorsPort;
/// let _: Option<&dyn OperatorsPort> = None;
/// ```
/// Implementations must supply the semantic evaluator.
///
/// ```compile_fail,E0046
/// use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
/// struct MissingEvaluator;
/// impl OperatorsUseCase for MissingEvaluator {}
/// ```
///
/// A borrowed request cannot retain its input beyond the invocation.
///
/// ```compile_fail
/// use ryo_operators::domain::contract::types::OperatorCall;
/// use ryo_relations::domain::relations::types::Value;
/// fn retain(call: OperatorCall<'_>) -> &'static [Value] {
///     match call {
///         OperatorCall::Project { row, .. } => row,
///         _ => &[],
///     }
/// }
/// ```
///
/// Pure row semantics without scheduling, retained storage or publication authority.
pub trait OperatorsUseCase: Sync {
    /// Evaluate one borrowed request into caller-owned scratch.
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault>;
}
