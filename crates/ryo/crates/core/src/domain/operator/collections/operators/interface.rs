//! Every primitive declares semantics and evaluates finite engine-prepared inputs.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::Operation;

/// No upstream scheduler, cache owner or retained state is a primitive capability.
pub(crate) trait OperatorsItem: Sync {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError>;
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        output: &mut dyn RowSink,
    ) -> Result<(), OperatingError>;
}
