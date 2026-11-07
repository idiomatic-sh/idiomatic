//! Typed positions share the owner-acquired buffer and refuse wrong views.
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;
type Failure = OperatingError;

#[test]
fn typed_positions_share_the_owner_acquired_buffer_and_refuse_wrong_views() -> Result<(), Failure> {
    use crate::application::file_dag::{derived::owner::ViewContext, relations::RelationOwner};
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    use {
        crate::domain::derived_views::types::*, crate::domain::operation_schema::types::InputShape,
        crate::domain::operator::contract::views::BorrowedRows,
        ryo_values::domain::rows::service::push_row,
    };

    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in [1, 1, 2] {
        push_row(&mut table, &[Cell::Number(Integer(value))]);
    }
    let schema = [InputShape {
        kinds: &table.kinds,
        tree: None,
    }];
    let distinct = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            crate::outbound::derived::factory::semantics(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::DistinctRepresentatives {
            columns: vec![ColumnIndex(0)],
        },
        &schema,
    )?);
    let order = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            crate::outbound::derived::factory::semantics(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::PartitionOrder {
            keys: vec![ColumnIndex(0)],
        },
        &schema,
    )?);
    let owner = RelationOwner::new(
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            crate::outbound::derived::factory::semantics(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let publication = owner.computed(table.clone())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let rows = BorrowedRows::from(&held);
    let proof = owner.admit(publication.as_ref(), rows)?;
    let acquired = owner.derived().acquire_stored(
        &distinct,
        &owner
            .derived()
            .admit_view(&distinct, &owner, &[&proof])
            .unwrap(),
        ViewContext::default(),
        DerivedInputs::Distinct {
            rows: proof.rows(),
            positions: &[0, 1, 2],
        },
        &mut DerivedScratch::default(),
        None,
    )?;
    let DerivedView::RowIds(ryo_derived::domain::positions::types::RowSlots(actual)) = acquired.view()? else {
        panic!("registered distinct view")
    };
    let original_buffer = actual.as_ptr();
    let shared = super::super::super::shared_positions(
        &acquired,
        super::super::super::RowBound(3),
        super::super::super::RowBound(3),
    )?
    .0;
    assert_eq!(shared.as_slice(), &[0, 2]);
    assert_eq!(shared.as_ptr(), original_buffer, "no row-ID buffer copy");
    assert!(matches!(
        super::super::super::shared_positions(
            &acquired,
            super::super::super::RowBound(2),
            super::super::super::RowBound(3)
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        super::super::super::shared_positions(
            &acquired,
            super::super::super::RowBound(3),
            super::super::super::RowBound(1)
        ),
        Err(OperatingError::ShapeMismatch)
    ));
    let wrong = owner.derived().acquire_stored(
        &order,
        &owner.derived().admit_view(&order, &owner, &[&proof]).unwrap(),
        ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )?;
    assert!(matches!(
        super::super::super::shared_positions(
            &wrong,
            super::super::super::RowBound(3),
            super::super::super::RowBound(3)
        ),
        Err(OperatingError::WrongOperation)
    ));
    Ok(())
}
