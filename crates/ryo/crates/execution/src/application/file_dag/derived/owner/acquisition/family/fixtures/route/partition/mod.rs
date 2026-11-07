//! The partition entry: families acquired through the `PartitionedClosure` cursor.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::{Exit, released};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::numbers;
use crate::application::file_dag::{
    relations::RelationOwner, fixtures::program_derived_port,
};
use crate::domain::{operating::errors::OperatingError, operator::contract::views::BorrowedRows};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::rc::Rc;

pub(super) fn partition(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(crate::application::file_dag::relations::types::ReuseEligibility::Eligible, program_derived_port()));
    let result = through_partition(kind, exit, &owner);
    owner.release_population();
    released(kind, exit, owner.derived().publication_ownership_bytes());
    result
}
fn through_partition(
    kind: DerivedKind,
    exit: Exit,
    owner: &Rc<RelationOwner>,
) -> Result<DerivedKind, OperatingError> {
    let publication = owner.computed(numbers())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let mut cursor = crate::application::file_dag::closure_partitions::schemas::prepare(
        &publication,
        BorrowedRows::from(&held),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        Rc::clone(owner),
    )?;
    match exit {
        Exit::Cancelled => Ok(kind),
        Exit::Succeeds => {
            assert_eq!(cursor.advance()?, crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced, "{kind:?}: the first partition is built");
            Ok(kind)
        }
        Exit::Errors => {
            owner.release_population();
            cursor.advance().map(|_| kind)
        }
    }
}

