//! Controls of the admission, acquisition and build entries of the derived owner.
use super::{Acquire, refuses};

pub(in super::super) static ADMIT_VIEW: Acquire = Acquire {
    positive: r#"
mod census_admit_view {
    use crate::application::file_dag::{derived::owner::{DerivedOwner, acquisition::admission::service::AdmittedViewInputs}, relations::{RelationOwner, admission::service::BorrowedInput}};
    use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand<'a>(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, proof: &BorrowedInput<'a>) -> Result<AdmittedViewInputs<'a>, OperatingError> { owner.admit_view(plan, relations, &[proof]) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_admit_view_denied {
    use crate::application::file_dag::{derived::owner::DerivedOwner, relations::RelationOwner};
    use crate::domain::{derived_views::types::DerivedContract, operator::contract::views::BorrowedRows};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, rows: BorrowedRows<'_>) { let _ = owner.admit_view(plan, relations, &[&rows]); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "BorrowedInput",
        "mismatched",
    ),
};
pub(in super::super) static ADMIT_PARTITION: Acquire = Acquire {
    positive: r#"
mod census_admit_partition {
    use crate::application::file_dag::{derived::owner::{DerivedOwner, acquisition::admission::service::{AcquiredView, AdmittedViewInputs}}, relations::{RelationOwner, admission::service::BorrowedInput}};
    use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand<'a>(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, proof: &BorrowedInput<'a>, parent: &AcquiredView) -> Result<AdmittedViewInputs<'a>, OperatingError> { owner.admit_partition(plan, relations, proof, parent) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_admit_partition_denied {
    use crate::application::file_dag::{derived::owner::DerivedOwner, relations::{RelationOwner, admission::service::BorrowedInput}};
    use crate::domain::derived_views::types::DerivedContract;
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, proof: &BorrowedInput<'_>, parent: &DerivedContract) { let _ = owner.admit_partition(plan, relations, proof, parent); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "AcquiredView",
        "mismatched",
    ),
};
pub(in super::super) static ACQUIRE_STORED: Acquire = Acquire {
    positive: r#"
mod census_acquire_stored {
    use crate::application::file_dag::derived::owner::{DerivedOwner, ViewContext, acquisition::{admission::service::{AcquiredView, AdmittedViewInputs}, selection::types::StoredColumns}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch, stored: Option<StoredColumns>) -> Result<AcquiredView, OperatingError> { owner.acquire_stored(plan, admitted, ViewContext::default(), inputs, scratch, stored) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_stored_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, ViewContext};
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) { let _ = owner.acquire_stored(plan, plan, ViewContext::default(), inputs, scratch, None); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "AdmittedViewInputs",
        "mismatched",
    ),
};
pub(in super::super) static ACQUIRE_STORED_WITH_ROUTE: Acquire = Acquire {
    positive: r#"
mod census_acquire_stored_with_route {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::{DerivedOwner, ViewContext, acquisition::{admission::service::{AcquiredView, AdmittedViewInputs}, selection::types::StoredColumns}}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch, stored: Option<StoredColumns>) -> Result<AcquiredView, OperatingError> { owner.acquire_stored_with_route(route, plan, admitted, ViewContext::default(), inputs, scratch, stored) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_stored_with_route_denied {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::{DerivedOwner, ViewContext}};
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) { let _ = owner.acquire_stored_with_route(route, plan, plan, ViewContext::default(), inputs, scratch, None); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "AdmittedViewInputs",
        "mismatched",
    ),
};
pub(in super::super) static TRY_ACQUIRE_ROWS: Acquire = Acquire {
    positive: r#"
mod census_try_acquire_rows {
    use crate::application::file_dag::derived::owner::{DerivedOwner, acquisition::admission::service::{AcquiredView, AdmittedViewInputs}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) -> Result<Option<AcquiredView>, OperatingError> { owner.try_acquire_rows(plan, admitted, inputs, scratch) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_try_acquire_rows_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) { let _ = owner.try_acquire_rows(plan, plan, inputs, scratch); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "AdmittedViewInputs",
        "mismatched",
    ),
};
pub(in super::super) static SCOPED_BUILD: Acquire = Acquire {
    positive: r#"
mod census_scoped_build {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::{DerivedOwner, acquisition::admission::service::{AcquiredView, AdmittedViewInputs}}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, recycled: Option<AcquiredView>, scratch: &mut DerivedScratch) -> Result<AcquiredView, OperatingError> { owner.scoped_build(route, plan, admitted, inputs, recycled, scratch) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_scoped_build_denied {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::DerivedOwner};
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) { let _ = owner.scoped_build(route, plan, plan, inputs, None, scratch); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "AdmittedViewInputs",
        "mismatched",
    ),
};
pub(in super::super) static FINITE_RAW: Acquire = Acquire {
    positive: r#"
mod census_finite_raw {
    use crate::application::file_dag::{relations::RelationOwner, program::runtime::service::KernelPermit};
    use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError};
    use std::{rc::Rc, sync::Arc};
    fn raw<'a>(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>, permit: &'a KernelPermit<'a>) -> Result<(), OperatingError> {
        let _ = owner.derived().finite_raw(Rc::clone(&owner), Some(plan), permit)?;
        Ok(())
    }
    const _: () = { let _ = raw; };
}
"#,
    negative: refuses(
        r#"
mod census_finite_raw_denied {
    use crate::application::file_dag::relations::RelationOwner;
    use crate::domain::derived_views::types::DerivedContract;
    use std::{rc::Rc, sync::Arc};
    fn forge(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>) { let _ = owner.derived().finite_raw(Rc::clone(&owner), Some(plan)); }
    const _: () = { let _ = forge; };
}
"#,
        "E0061",
        "KernelPermit",
        "argument",
    ),
};
pub(in super::super) static WITH_RAW_ORDER: Acquire = Acquire {
    positive: r#"
mod census_with_raw_order {
    use crate::application::file_dag::derived::owner::raw_scope::RawScope;
    use crate::domain::{operating::errors::OperatingError, positions::types::{Keys, Stride}};
    fn order(scope: &mut RawScope, keys: &Keys) -> Result<(), OperatingError> { scope.with_raw_order(keys, Stride(1), &mut |_order| Ok(())) }
    const _: () = { let _ = order; };
}
"#,
    negative: refuses(
        r#"
mod census_with_raw_order_denied {
    use crate::application::file_dag::{derived::owner::raw_scope::RawScope, relations::RelationOwner};
    use std::rc::Rc;
    fn forge(owner: Rc<RelationOwner>) -> RawScope { RawScope::for_program(owner, None, None) }
    const _: () = { let _ = forge; };
}
"#,
        "E0624",
        "for_program",
        "private",
    ),
};
pub(in super::super) static EVALUATE_NATIVE: Acquire = Acquire {
    positive: r#"
mod census_evaluate_native {
    use crate::application::file_dag::derived::owner::raw_scope::RawScope;
    const _: () = { let _ = RawScope::evaluate_native; };
}
"#,
    negative: refuses(
        r#"
mod census_evaluate_native_denied {
    use crate::application::file_dag::derived::owner::raw_scope::RawScope;
    fn peek(scope: &RawScope) { let _ = &scope.working; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "working",
        "private",
    ),
};
