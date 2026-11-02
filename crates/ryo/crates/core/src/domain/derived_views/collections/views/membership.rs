//! Duplicate-insensitive membership over values, dense graph order or native positions.
use super::interface::DerivedViewsItem;
use crate::domain::{
    derived_views::{service, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    table_cells::types::RowIndex,
};
use ryo_language::domain::relations::{
    service::{node_file, node_position},
    types::Value,
};
pub(crate) struct MembershipItem;
impl DerivedViewsItem for MembershipItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::Membership, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::Membership, contract)?;
        let (ViewRequest::Membership { column }, DerivedInputs::Membership { rows, space }) =
            (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let mut values = hashbrown::HashSet::new();
        for at in 0..rows.count() {
            let value = rows
                .cell(RowIndex(at), *column)
                .ok_or(OperatingError::ShapeMismatch)?;
            let _new = values.insert(value);
        }
        let membership = match space {
            MembershipSpace::Values => MembershipData::Values(values),
            MembershipSpace::Dense(vertices) => MembershipData::Dense(
                vertices
                    .iter()
                    .map(|vertex| values.contains(vertex))
                    .collect(),
            ),
            MembershipSpace::Native(tree) => {
                let mut marks = vec![false; tree.elements.kind.len()];
                for value in values {
                    if let Value::Node(node) = value {
                        if node_file(node) == tree.file {
                            if let Some(mark) = marks.get_mut(node_position(node).0 as usize) {
                                *mark = true;
                            }
                        }
                    }
                }
                MembershipData::Dense(marks)
            }
        };
        *storage = DerivedStorage::Membership(membership);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::Membership, contract, storage)
    }
}
