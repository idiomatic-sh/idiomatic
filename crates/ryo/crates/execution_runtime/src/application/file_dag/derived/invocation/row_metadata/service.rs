//! Join resource metadata is checked before any payload is built.
use super::super::prepared_use::service::prepared;
use super::super::service::input;
use super::super::types::InvocationResources;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::{
    resources::types::ResourceProgram, types::OperatorContract,
};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_values::domain::plan_parts::types::InputNumber;

impl InvocationResources<'_> {
    /// Validate required Join resource metadata without constructing a payload.
    /// Availability, geometry and static output checks remain at their existing
    /// owner boundaries; an optional unsupported native lookup still declines.
    pub(crate) fn validate_row_metadata(
        contract: &OperatorContract,
        inputs: &[RepresentationInput<'_>],
    ) -> Result<(), OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        let ResourceProgram::Join(join) = &contract.resources else {
            return Ok(());
        };
        let right = input(inputs, InputNumber(1))?;
        let validate = |kind| {
            let request = prepared(contract, kind)?;
            ryo_execution_domain::domain::derived_views::service::validate(kind, &request.plan)
        };
        if join.reach.is_some() {
            validate(DerivedKind::PartitionOrder)?;
            validate(DerivedKind::ClosureAdjacency)
        } else if let RepresentationInput::Tree(tree) = right {
            validate(DerivedKind::NativeLookup)?;
            let native = prepared(contract, DerivedKind::NativeLookup)?;
            let form = match tree.form {
                ryo_execution_domain::domain::table_cells::types::TreeForm::Elements => {
                    ryo_derived::domain::native::types::NativeForm::Elements
                }
                ryo_execution_domain::domain::table_cells::types::TreeForm::Edges => {
                    ryo_derived::domain::native::types::NativeForm::Edges
                }
            };
            if ryo_derived::domain::dispatch::service::native_finding(&native.plan.semantic, form)
                .map_err(ryo_execution_domain::domain::operating::service::derived_fault)?
                .is_some()
            {
                return Ok(());
            }
            if join.has_filter
                == ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        } else {
            // The ordinary values driver has no native filter/view protocol.
            if join.has_filter
                == ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Filtered
                || !join.view.is_empty()
            {
                return Err(OperatingError::WrongOperation);
            }
            validate(DerivedKind::Equality)
        }
    }
}
