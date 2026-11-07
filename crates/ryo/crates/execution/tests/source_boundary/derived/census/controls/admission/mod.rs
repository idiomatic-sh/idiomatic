//! Controls of the admission, acquisition and build entries of the derived owner.
use super::{Acquire, refuses};

pub(in super::super) static ADMIT_VIEW: Acquire = Acquire {
    positive: r#"
mod census_admit_view {
    use crate::application::file_dag::{derived::owner::{DerivedOwner, acquisition::admission::service::AdmittedViewInputs}, relations::{RelationOwner, admission::service::BorrowedInput}};
    use crate::domain::{derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand<'a>(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, proof: &BorrowedInput<'a>) -> Result<AdmittedViewInputs<'a>, OperatingError> { owner.admit_view(plan, relations, &[proof]) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_admit_view_denied {
    use crate::application::file_dag::{derived::owner::DerivedOwner, relations::RelationOwner};
    use crate::domain::derived_views::types::contract::value::DerivedContract;
    use crate::domain::table_model::types::RepresentationInput;
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, rows: RepresentationInput<'_>) { let _ = owner.admit_view(plan, relations, &[&rows]); }
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
    use crate::domain::{derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand<'a>(owner: &DerivedOwner, plan: &Arc<DerivedContract>, relations: &RelationOwner, proof: &BorrowedInput<'a>, parent: &AcquiredView) -> Result<AdmittedViewInputs<'a>, OperatingError> { owner.admit_partition(plan, relations, proof, parent) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_admit_partition_denied {
    use crate::application::file_dag::{derived::owner::DerivedOwner, relations::{RelationOwner, admission::service::BorrowedInput}};
    use crate::domain::derived_views::types::contract::value::DerivedContract;
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
    use crate::application::file_dag::derived::owner::{DerivedOwner, ViewContext, acquisition::{admission::service::{AcquiredView, AdmittedViewInputs}, population::types::StoredColumns}};
    use crate::domain::{derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch, stored: Option<StoredColumns>) -> Result<AcquiredView, OperatingError> { owner.acquire_stored(plan, admitted, ViewContext::default(), inputs, scratch, stored) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_stored_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, ViewContext};
    use crate::domain::derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch};
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
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::{DerivedOwner, ViewContext, acquisition::{admission::service::{AcquiredView, AdmittedViewInputs}, population::types::StoredColumns}}};
    use crate::domain::{derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch, stored: Option<StoredColumns>) -> Result<AcquiredView, OperatingError> { owner.acquire_stored_with_route(route, plan, admitted, ViewContext::default(), inputs, scratch, stored) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_stored_with_route_denied {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::{DerivedOwner, ViewContext}};
    use crate::domain::derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch};
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
    use crate::domain::{derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) -> Result<Option<AcquiredView>, OperatingError> { owner.try_acquire_rows(plan, admitted, inputs, scratch) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_try_acquire_rows_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    use crate::domain::derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch};
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
    use crate::domain::{derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, recycled: Option<AcquiredView>, scratch: &mut DerivedScratch) -> Result<AcquiredView, OperatingError> { owner.scoped_build(route, plan, admitted, inputs, recycled, scratch) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_scoped_build_denied {
    use crate::application::file_dag::{route_counts::types::BuildRoute, derived::owner::DerivedOwner};
    use crate::domain::derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch};
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
pub(in super::super) static PREPARE_PROGRAM_FILE: Acquire = Acquire {
    positive: r#"
mod census_prepare_program_file {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::prepare_program_file; };
}
"#,
    negative: refuses(
        r#"
mod census_prepare_program_file_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn peek(owner: &DerivedOwner) { let _ = &owner.file_prepared; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "file_prepared",
        "private",
    ),
};
