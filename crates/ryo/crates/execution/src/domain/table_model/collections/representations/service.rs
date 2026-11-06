//! Exhaustive registration; no representation can opt out of the contract.
use super::types::{RepresentationInput, RepresentationKind};
use crate::domain::table_model::types::Rows;

pub(crate) use super::registration::registered;

pub(crate) fn kind(input: RepresentationInput<'_>) -> RepresentationKind {
    super::types::registered_representation::value::RegisteredRepresentation::kind(input)
}
pub(crate) fn borrowed<'a>(rows: &'a Rows<'a>) -> RepresentationInput<'a> {
    match rows {
        Rows::Stored(source) => RepresentationInput::Stored(*source),
        Rows::Selected(view) => RepresentationInput::Selected {
            source: view.source,
            positions: &view.selection.rows,
            columns: &view.selection.columns,
            kinds: &view.selection.kinds,
        },
        Rows::Tree(tree) => RepresentationInput::Tree(*tree),
        Rows::Computed(table) => RepresentationInput::Computed(table),
        Rows::Compact(table) => RepresentationInput::Compact(table),
        Rows::Positioned(view) => RepresentationInput::Positioned(*view),
    }
}

impl<'f> From<&'f Rows<'f>> for RepresentationInput<'f> {
    fn from(rows: &'f Rows<'f>) -> Self {
        borrowed(rows)
    }
}
impl<'f> RepresentationInput<'f> {
    pub(crate) fn bulk(
        self,
    ) -> Result<super::types::BulkAccess<'f>, crate::domain::operating::errors::OperatingError>
    {
        registered(kind(self)).bulk(self)
    }

    pub(crate) fn contract(
        self,
    ) -> Result<super::types::RelationContract<'f>, crate::domain::operating::errors::OperatingError>
    {
        registered(kind(self)).prepare(self)
    }
    pub(crate) fn validate(self) -> Result<(), crate::domain::operating::errors::OperatingError> {
        let contract = self.contract()?;
        if registered(contract.kind).inspect(&contract)? == super::types::PhysicalShape::Complete {
            Ok(())
        } else {
            Err(crate::domain::operating::errors::OperatingError::ShapeMismatch)
        }
    }
    // Metadata-only compatibility for existing domain algorithms. Validation is
    // never triggered by a cardinality/schema request.
    pub(crate) fn count(self) -> usize {
        match self {
            Self::Stored(view) => view.columns.first().map_or(0, |column| column.0.len()),
            Self::Selected { positions, .. } => positions.len(),
            Self::Tree(tree) => tree.elements.kind.len().saturating_sub(usize::from(
                tree.form == crate::domain::table_cells::types::TreeForm::Edges,
            )),
            Self::Computed(table) => table.count.0,
            Self::Compact(table) => ryo_values::domain::table_words::service::compact_rows(table).0,
            Self::Positioned(view) => view
                .positions
                .slots
                .0
                .len()
                .checked_div(view.positions.width.0)
                .unwrap_or(0),
        }
    }
    pub(crate) fn kinds(self) -> &'f [ryo_relations::domain::relations::types::CellKind] {
        match self {
            Self::Stored(view) => view.kinds,
            Self::Selected { kinds, .. } => kinds,
            Self::Positioned(view) => view.kinds,
            Self::Computed(table) => &table.kinds,
            Self::Compact(table) => ryo_values::domain::table_words::service::compact_kinds(table),
            Self::Tree(tree) => ryo_relations::domain::relations::service::kinds(match tree.form {
                crate::domain::table_cells::types::TreeForm::Elements => {
                    ryo_relations::domain::relations::types::Relation::SyntaxTree
                }
                crate::domain::table_cells::types::TreeForm::Edges => {
                    ryo_relations::domain::relations::types::Relation::SyntaxChild
                }
            }),
        }
    }
    pub(crate) fn cell(
        self,
        row: ryo_values::domain::rows::types::RowIndex,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value> {
        match registered(kind(self)).cell(self, row, column).ok()? {
            super::types::CellRead::Value(value) => Some(value),
            super::types::CellRead::MissingPhysical => None,
        }
    }
    pub(crate) fn read<'r>(
        self,
        row: ryo_values::domain::rows::types::RowIndex,
        decode: &'r mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> Option<&'r [ryo_relations::domain::relations::types::Value]>
    where
        'f: 'r,
    {
        if !matches!(self, Self::Computed(_)) {
            decode.resize(
                self.kinds().len(),
                ryo_relations::domain::relations::types::Value::Flag(
                    ryo_relations::domain::relations::types::Flag::Lowered,
                ),
            );
        }
        match registered(kind(self)).row(self, row, decode).ok()? {
            super::types::RowRead::Borrowed(row) | super::types::RowRead::Decoded(row) => Some(row),
            super::types::RowRead::MissingPhysical => None,
        }
    }
}
