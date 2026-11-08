//! The sole registered invocation representation retains private construction.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::{PreparedInputs, PrimitiveLease, RowSink};
/// The executable lease belongs to the application invocation owner. A pure
/// borrowed context cannot be promoted into a registered call by a sibling.
pub(crate) struct ExecutableLease<'call, 'resources, 'scope> {
    permit: &'call crate::application::file_dag::program::runtime::service::KernelPermit<'scope>,
    context: &'call mut PrimitiveLease<'resources>,
    operation: &'call ryo_values::domain::plans::types::Operation,
    input: PreparedInputs<'call>,
    output: &'call mut dyn RowSink,
}
impl<'call, 'resources, 'scope> ExecutableLease<'call, 'resources, 'scope> {
    pub(in crate::application::file_dag::execution::lease) fn new(
        permit: &'call crate::application::file_dag::program::runtime::service::KernelPermit<
            'scope,
        >,
        context: &'call mut PrimitiveLease<'resources>,
        operation: &'call ryo_values::domain::plans::types::Operation,
        input: PreparedInputs<'call>,
        output: &'call mut dyn RowSink,
    ) -> Self {
        Self {
            permit,
            context,
            operation,
            input,
            output,
        }
    }

    pub(in crate::application::file_dag::execution) fn dispatch(
        self,
    ) -> Result<(), OperatingError> {
        self.permit.validate_registered(self.operation)?;
        ryo_execution_domain::domain::operator::contract::registration::registered(self.permit.kind()).evaluate(
            self.operation,
            self.input,
            self.context,
            self.output,
        )
    }
}
