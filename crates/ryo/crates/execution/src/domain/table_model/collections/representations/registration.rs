//! Every physical implementation is reached only through its classified catalog entry.
use super::{interface::RepresentationsItem, types::*};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::RowIndex};

impl RegisteredRepresentation {
    fn require(&self, input: RepresentationInput<'_>) -> Result<(), OperatingError> {
        if RepresentationKind::of(input) != self.0 {
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
        if contract.kind != self.0 {
            return Err(OperatingError::WrongOperation);
        }
        Ok(contract)
    }
    pub(crate) fn inspect(
        &self,
        contract: &RelationContract<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        if contract.kind != self.0 {
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
        match self.0 {
            RepresentationKind::Stored => &super::stored::item::StoredItem,
            RepresentationKind::Selected => &super::selected::item::SelectedItem,
            RepresentationKind::Tree => &super::tree::item::TreeItem,
            RepresentationKind::Computed => &super::computed::item::ComputedItem,
            RepresentationKind::Compact => &super::compact::item::CompactItem,
            RepresentationKind::Positioned => &super::positioned::item::PositionedItem,
        }
    }
}

pub(crate) fn registered(kind: RepresentationKind) -> RegisteredRepresentation {
    RegisteredRepresentation(kind)
}
