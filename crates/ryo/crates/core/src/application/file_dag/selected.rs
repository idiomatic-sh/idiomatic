//! Generation-local selected-row index and direct projected-cell emission.
//! Only the application retains selection metadata; primitives receive borrowed typed rows.

use super::derived::view::AcquiredView;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::table_model::types::SelectedRows;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

/// One complete selected population and an index over only its declared key cells.
/// The source remains borrowed from this physical file; metadata is an Arc clone.
pub(crate) struct SelectedInput<'f> {
    original: SelectedRows<'f>,
    index: AcquiredView,
}
impl<'f> SelectedInput<'f> {
    #[cfg(test)]
    pub(crate) fn prepare(
        original: &SelectedRows<'f>,
        columns: &[ColumnIndex],
    ) -> Result<Self, OperatingError> {
        use crate::domain::derived_views::types::*;
        let rows = BorrowedRows::Selected {
            source: original.source,
            positions: &original.selection.rows,
            columns: &original.selection.columns,
            kinds: &original.selection.kinds,
        };
        rows.validate()?;
        let plan = std::sync::Arc::new(super::derived::prepare::view(
            crate::outbound::derived::factory::test_port(),
            &ViewRequest::Equality {
                columns: columns.to_vec(),
            },
            &[crate::domain::operation_schema::types::InputShape {
                kinds: rows.kinds(),
                tree: None,
            }],
        )?);
        let owner = super::relations::RelationOwner::new(
            false,
            crate::outbound::derived::factory::test_port(),
        );
        let proof = owner.admit_external(rows)?;
        let (population, _) = proof.identity(&owner)?;
        let index = owner.derived.acquire_stored(
            &plan,
            &[population],
            Default::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            None,
        )?;
        Self::from_derived(original, index)
    }
    pub(super) fn from_derived(
        original: &SelectedRows<'f>,
        index: AcquiredView,
    ) -> Result<Self, OperatingError> {
        if !matches!(
            index.view()?,
            ryo_derived::domain::contract::types::DerivedView::Equality(_)
        ) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(Self {
            original: original.clone(),
            index,
        })
    }
    fn columns(&self) -> &[ColumnIndex] {
        let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
            &self.index.contract().request
        else {
            unreachable!("prepared equality")
        };
        columns
    }
    fn index(&self) -> &KeyIndex {
        let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
            self.index.view().expect("validated equality view")
        else {
            unreachable!("validated equality view")
        };
        index
    }
    pub(crate) fn view(&self) -> IndexedRows<'_> {
        IndexedRows {
            #[cfg(feature = "diagnostic-row-progress")]
            progress: crate::application::file_dag::route_counts::progress,
            rows: BorrowedRows::Selected {
                source: self.original.source,
                positions: &self.original.selection.rows,
                columns: &self.original.selection.columns,
                kinds: &self.original.selection.kinds,
            },
            columns: self.columns(),
            index: self.index(),
        }
    }

    /// Join one left row with matching compact positions in their original bag order.
    /// Only output cells are decoded; no full RHS row or table is materialized.
    pub(crate) fn emit_join(
        &self,
        semantics: &dyn crate::domain::operator::contract::types::PrimitiveSemantics,
        left_on: &[ColumnIndex],
        output: &[JoinColumn],
        left: &[Value],
        key: &mut Vec<Value>,
        answer: &mut Vec<Value>,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        super::equality_join::indexed(
            semantics,
            self.view(),
            left,
            left_on,
            output,
            key,
            answer,
            context,
            emit,
        )
    }
}

#[cfg(test)]
mod tests;

use ryo_derived::domain::contract::types::DerivedKind;
