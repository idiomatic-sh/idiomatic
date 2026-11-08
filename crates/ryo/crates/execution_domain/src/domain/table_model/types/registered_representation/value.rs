//! A classified physical catalog handle; implementer storage never crosses this value.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::collections::representations::{
    compact::item::CompactItem,
    computed::item::ComputedItem,
    interface::RepresentationsItem,
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
/// The representation registered for one kind, through which its rows are prepared and inspected.
pub struct RegisteredRepresentation {
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
            RepresentationInput::Tree(_) => RepresentationKind::Tree,
            RepresentationInput::Computed(_) => RepresentationKind::Computed,
            RepresentationInput::Compact(_) => RepresentationKind::Compact,
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
    /// Whether the contract's rows can be read in full.
    pub fn inspect(
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
            RepresentationKind::Tree => &TreeItem,
            RepresentationKind::Computed => &ComputedItem,
            RepresentationKind::Compact => &CompactItem,
        }
    }
}

impl<'f> RepresentationInput<'f> {
    /// The rows as one bulk representation.
    pub fn bulk(
        self,
    ) -> Result<RepresentationInput<'f>, OperatingError>
    {
        RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).bulk(self)
    }
    /// The contract the representation declares of its rows.
    pub fn contract(
        self,
    ) -> Result<RelationContract<'f>, OperatingError>
    {
        RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).prepare(self)
    }
    /// Checks that the rows' contract can be read in full.
    pub fn validate(self) -> Result<(), OperatingError> {
        let contract = self.contract()?;
        if RegisteredRepresentation::new(contract.kind).inspect(&contract)? == PhysicalShape::Complete {
            Ok(())
        } else {
            Err(OperatingError::ShapeMismatch)
        }
    }
    /// One cell of the rows, or nothing when it is not physically present.
    pub fn cell(
        self,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Option<Value> {
        match RegisteredRepresentation::new(RegisteredRepresentation::kind(self)).cell(self, row, column) {
            Ok(CellRead::Value(value)) => Some(value),
            Ok(CellRead::MissingPhysical) | Err(_) => None,
        }
    }
    /// One row of the rows, read into the buffer, or nothing when it is not physically present.
    pub fn read<'r>(
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
    /// How many rows there are.
    pub fn count(self) -> RowCount {
        match self {
            Self::Stored(view) => RowCount(view.columns.first().map_or(0, |column| column.0.len())),
            Self::Tree(tree) => RowCount(tree.elements.kind.len().saturating_sub(usize::from(
                tree.form == TreeForm::Edges,
            ))),
            Self::Computed(table) => table.count,
            Self::Compact(table) => ryo_values::domain::compact_table::service::compact_rows(table),
        }
    }
    /// The cell kinds of the columns.
    pub fn kinds(self) -> &'f [CellKind] {
        match self {
            Self::Stored(view) => view.kinds,
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
