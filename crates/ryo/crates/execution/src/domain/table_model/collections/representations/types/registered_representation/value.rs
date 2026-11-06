//! A classified physical catalog handle; implementer storage never crosses this value.
use super::super::super::{
    interface::RepresentationsItem,
    types::{
        BulkAccess, CellRead, PhysicalShape, RelationContract, RepresentationInput,
        RepresentationKind, RowRead,
    },
};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::RowIndex};
pub(crate) struct RegisteredRepresentation {
    kind: RepresentationKind,
}
impl RegisteredRepresentation {
    pub(in crate::domain::table_model::collections::representations) fn new(
        kind: RepresentationKind,
    ) -> Self {
        Self { kind }
    }
}

impl RegisteredRepresentation {
    pub(in crate::domain::table_model::collections::representations) fn kind(
        input: RepresentationInput<'_>,
    ) -> RepresentationKind {
        match input {
            RepresentationInput::Stored(_) => RepresentationKind::Stored,
            RepresentationInput::Selected { .. } => RepresentationKind::Selected,
            RepresentationInput::Tree(_) => RepresentationKind::Tree,
            RepresentationInput::Computed(_) => RepresentationKind::Computed,
            RepresentationInput::Compact(_) => RepresentationKind::Compact,
            RepresentationInput::Positioned(_) => RepresentationKind::Positioned,
        }
    }
}

impl RegisteredRepresentation {
    fn require(&self, input: RepresentationInput<'_>) -> Result<(), OperatingError> {
        if Self::kind(input) != self.kind {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
    pub(crate) fn prepare<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<RelationContract<'f>, OperatingError> {
        self.require(input)?;
        let contract = self.item().prepare(input)?;
        if contract.kind != self.kind {
            return Err(OperatingError::WrongOperation);
        }
        Ok(contract)
    }
    pub(crate) fn inspect(
        &self,
        contract: &RelationContract<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        if contract.kind != self.kind {
            return Err(OperatingError::WrongOperation);
        }
        self.require(contract.input)?;
        self.item().inspect(contract)
    }
    pub(crate) fn bulk<'f>(
        &self,
        input: RepresentationInput<'f>,
    ) -> Result<BulkAccess<'f>, OperatingError> {
        self.require(input)?;
        self.item().bulk(input)
    }
    pub(crate) fn cell(
        &self,
        input: RepresentationInput<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, OperatingError> {
        self.require(input)?;
        self.item().cell(input, row, column)
    }
    pub(crate) fn row<'f>(
        &self,
        input: RepresentationInput<'f>,
        row: RowIndex,
        decode: &'f mut [ryo_relations::domain::relations::types::Value],
    ) -> Result<RowRead<'f>, OperatingError> {
        self.require(input)?;
        self.item().row(input, row, decode)
    }
}
impl RegisteredRepresentation {
    fn item(&self) -> &'static dyn RepresentationsItem {
        match self.kind {
            RepresentationKind::Stored => &super::super::super::stored::item::StoredItem,
            RepresentationKind::Selected => &super::super::super::selected::item::SelectedItem,
            RepresentationKind::Tree => &super::super::super::tree::item::TreeItem,
            RepresentationKind::Computed => &super::super::super::computed::item::ComputedItem,
            RepresentationKind::Compact => &super::super::super::compact::item::CompactItem,
            RepresentationKind::Positioned => {
                &super::super::super::positioned::item::PositionedItem
            }
        }
    }
}
