//! A classified physical catalog handle; implementer storage never crosses this value.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::collections::representations::{
    compact::item::CompactItem,
    computed::item::ComputedItem,
    interface::RepresentationsItem,
    positioned::item::PositionedItem,
    selected::item::SelectedItem,
    stored::item::StoredItem,
    tree::item::TreeItem,
    types::{CellRead, PhysicalShape, RelationContract, RepresentationKind, RowRead},
};
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_cells::types::TreeForm;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, RowIndex},
};
pub(crate) struct RegisteredRepresentation {
    kind: RepresentationKind,
}
impl RegisteredRepresentation {
    pub(crate) fn new(
        kind: RepresentationKind,
    ) -> Self {
        Self { kind }
    }
}

impl RegisteredRepresentation {
    pub(crate) fn kind(
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
    ) -> Result<RepresentationInput<'f>, OperatingError> {
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
        decode: &'f mut [Value],
    ) -> Result<RowRead<'f>, OperatingError> {
        self.require(input)?;
        self.item().row(input, row, decode)
    }
}
impl RegisteredRepresentation {
    fn item(&self) -> &'static dyn RepresentationsItem {
        match self.kind {
            RepresentationKind::Stored => &StoredItem,
            RepresentationKind::Selected => &SelectedItem,
            RepresentationKind::Tree => &TreeItem,
            RepresentationKind::Computed => &ComputedItem,
            RepresentationKind::Compact => &CompactItem,
            RepresentationKind::Positioned => &PositionedItem,
        }
    }
}

impl<'f> RepresentationInput<'f> {
    pub(crate) fn bulk(
        self,
    ) -> Result<RepresentationInput<'f>, OperatingError>
    {
        RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).bulk(self)
    }
    pub(crate) fn contract(
        self,
    ) -> Result<RelationContract<'f>, OperatingError>
    {
        RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).prepare(self)
    }
    pub(crate) fn validate(self) -> Result<(), OperatingError> {
        let contract = self.contract()?;
        if crate::domain::table_model::service::registered(contract.kind).inspect(&contract)? == PhysicalShape::Complete {
            Ok(())
        } else {
            Err(OperatingError::ShapeMismatch)
        }
    }
    pub(crate) fn cell(
        self,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Option<Value> {
        match RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).cell(self, row, column) {
            Ok(CellRead::Value(value)) => Some(value),
            Ok(CellRead::MissingPhysical) | Err(_) => None,
        }
    }
    pub(crate) fn read<'r>(
        self,
        row: RowIndex,
        decode: &'r mut Vec<Value>,
    ) -> Option<&'r [Value]>
    where
        'f: 'r,
    {
        if !matches!(self, Self::Computed(_)) {
            decode.resize(
                self.kinds().len(),
                Value::Flag(
                    ryo_relations::domain::relations::types::Flag::Lowered,
                ),
            );
        }
        match RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).row(self, row, decode) {
            Ok(RowRead::Borrowed(row) | RowRead::Decoded(row)) => {
                Some(row)
            }
            Ok(RowRead::MissingPhysical) | Err(_) => None,
        }
    }
    // Metadata-only compatibility for existing domain algorithms. Validation is
    // never triggered by a cardinality/schema request.
    pub(crate) fn count(self) -> RowCount {
        match self {
            Self::Stored(view) => RowCount(view.columns.first().map_or(0, |column| column.0.len())),
            Self::Selected { positions, .. } => RowCount(positions.0.len()),
            Self::Tree(tree) => RowCount(tree.elements.kind.len().saturating_sub(usize::from(
                tree.form == TreeForm::Edges,
            ))),
            Self::Computed(table) => table.count,
            Self::Compact(table) => ryo_values::domain::compact_table::service::compact_rows(table),
            Self::Positioned(view) => RowCount(
                view.positions
                    .slots
                    .0
                    .len()
                    .checked_div(view.positions.width.0)
                    .unwrap_or(0),
            ),
        }
    }
    pub(crate) fn kinds(self) -> &'f [CellKind] {
        match self {
            Self::Stored(view) => view.kinds,
            Self::Selected { kinds, .. } => kinds,
            Self::Positioned(view) => view.kinds,
            Self::Computed(table) => &table.kinds,
            Self::Compact(table) => {
                ryo_values::domain::compact_table::service::compact_kinds(table)
            }
            Self::Tree(tree) => ryo_relations::domain::relations::service::kinds(match tree.form {
                TreeForm::Elements => {
                    Relation::SyntaxTree
                }
                TreeForm::Edges => {
                    Relation::SyntaxChild
                }
            }),
        }
    }
}
