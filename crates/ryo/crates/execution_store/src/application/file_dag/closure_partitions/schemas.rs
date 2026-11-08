//! The one place a prepared partition cursor is admitted: its plans, ordering and proof are checked here.
use super::{PartitionedClosure, types::OrderCursor};
use crate::application::file_dag::{
    derived::owner::acquisition::admission::service::AcquiredView, relations::RelationOwner,
};
use ryo_execution_domain::domain::{
    derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use std::{rc::Rc, sync::Arc};

pub(in crate::application::file_dag) fn from_prepared<'f>(
    proof: crate::application::file_dag::relations::admission::service::BorrowedInput<'f>,
    partition_plan: Arc<DerivedContract>,
    closure_plan: Arc<DerivedContract>,
    ordered: AcquiredView,
    owner: Rc<RelationOwner>,
) -> Result<PartitionedClosure<'f>, OperatingError> {
    ryo_execution_domain::domain::derived_views::service::validate(
        ryo_derived::domain::request::types::DerivedKind::PartitionOrder,
        &partition_plan,
    )?;
    ryo_execution_domain::domain::derived_views::service::validate(
        ryo_derived::domain::request::types::DerivedKind::ClosureAdjacency,
        &closure_plan,
    )?;
    if !std::ptr::eq(ordered.contract(), partition_plan.as_ref())
        || !matches!(
            ordered.view()?,
            ryo_derived::domain::contract::types::DerivedView::PartitionOrder(_)
        )
    {
        return Err(OperatingError::WrongOperation);
    }
    let input: RepresentationInput<'f> = proof.rows();
    owner
        .derived()
        .admit_partition(&closure_plan, &owner, &proof, &ordered)?;
    #[cfg(test)]
    let ordering_capacity = ordered.partition_order_capacity()?;
    Ok(PartitionedClosure {
        input,
        proof,
        owner,
        partition_plan,
        closure_plan,
        ordered,
        #[cfg(test)]
        ordering_capacity,
        cursor: OrderCursor(0),
        key: Vec::new(),
        current: None,
        scratch: ryo_execution_domain::domain::derived_views::types::DerivedScratch::default(),
    })
}
/// A partitioned closure over a publication, for the proofs of its cursor.
#[cfg(any(test, feature = "fixtures"))]
pub fn prepare<'f>(
    publication: &crate::application::file_dag::relations::handle::service::PublishedInput,
    input: RepresentationInput<'f>,
    keys: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    source: ryo_values::domain::plan_parts::types::ColumnIndex,
    destination: ryo_values::domain::plan_parts::types::ColumnIndex,
    owner: Rc<RelationOwner>,
) -> Result<PartitionedClosure<'f>, OperatingError> {
    use ryo_execution_domain::domain::derived_views::types::{DerivedInputs, DerivedScratch, ViewRequest};
    input.validate()?;
    let schema = [ryo_execution_domain::domain::operation_schema::types::InputShape {
        kinds: input.kinds(),
        tree: None,
    }];
    let partition_plan = Arc::new(owner.derived().prepare_view(
        &ViewRequest::PartitionOrder {
            keys: keys.to_vec(),
        },
        &schema,
    )?);
    let closure_plan = Arc::new(owner.derived().prepare_view(
        &ViewRequest::ClosureAdjacency {
            source,
            destination,
        },
        &schema,
    )?);
    let proof = owner.admit(publication.as_ref(), input)?;
    let admitted = owner
        .derived()
        .admit_view(&partition_plan, &owner, &[&proof])?;
    let ordered = owner.derived().acquire_stored(
        &partition_plan,
        &admitted,
        crate::application::file_dag::derived::owner::ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )?;
    from_prepared(proof, partition_plan, closure_plan, ordered, owner)
}
