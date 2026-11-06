//! One positive and one negative compile control per acquiring entry, and one forgery control
//! per authority a consumer may hold. Every source is inserted into the unmodified derived root.

pub(super) struct Negative {
    pub(super) source: &'static str,
    pub(super) code: &'static str,
    pub(super) first: &'static str,
    pub(super) second: &'static str,
}
pub(super) struct Acquire {
    pub(super) positive: &'static str,
    pub(super) negative: Negative,
}
const fn refuses(
    source: &'static str,
    code: &'static str,
    first: &'static str,
    second: &'static str,
) -> Negative {
    Negative {
        source,
        code,
        first,
        second,
    }
}

pub(super) static ADMIT_VIEW: Acquire = Acquire {
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
pub(super) static ADMIT_PARTITION: Acquire = Acquire {
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
pub(super) static ACQUIRE_STORED: Acquire = Acquire {
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
pub(super) static ACQUIRE_STORED_WITH_ROUTE: Acquire = Acquire {
    positive: r#"
mod census_acquire_stored_with_route {
    use crate::application::file_dag::{route_counts::BuildRoute, derived::owner::{DerivedOwner, ViewContext, acquisition::{admission::service::{AcquiredView, AdmittedViewInputs}, selection::types::StoredColumns}}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch, stored: Option<StoredColumns>) -> Result<AcquiredView, OperatingError> { owner.acquire_stored_with_route(route, plan, admitted, ViewContext::default(), inputs, scratch, stored) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_stored_with_route_denied {
    use crate::application::file_dag::{route_counts::BuildRoute, derived::owner::{DerivedOwner, ViewContext}};
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
pub(super) static TRY_ACQUIRE_ROWS: Acquire = Acquire {
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
pub(super) static SCOPED_BUILD: Acquire = Acquire {
    positive: r#"
mod census_scoped_build {
    use crate::application::file_dag::{route_counts::BuildRoute, derived::owner::{DerivedOwner, acquisition::admission::service::{AcquiredView, AdmittedViewInputs}}};
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn demand(owner: &DerivedOwner, route: BuildRoute, plan: &Arc<DerivedContract>, admitted: &AdmittedViewInputs<'_>, inputs: DerivedInputs<'_, '_>, recycled: Option<AcquiredView>, scratch: &mut DerivedScratch) -> Result<AcquiredView, OperatingError> { owner.scoped_build(route, plan, admitted, inputs, recycled, scratch) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_scoped_build_denied {
    use crate::application::file_dag::{route_counts::BuildRoute, derived::owner::DerivedOwner};
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
pub(super) static FINITE_RAW: Acquire = Acquire {
    positive: r#"
mod census_finite_raw {
    use crate::application::file_dag::{relations::RelationOwner, program::runtime::service::KernelPermit};
    use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError};
    use std::{rc::Rc, sync::Arc};
    fn raw<'a>(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>, permit: &'a KernelPermit<'a>) -> Result<(), OperatingError> {
        let _ = owner.derived.finite_raw(Rc::clone(&owner), Some(plan), permit)?;
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
    fn forge(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>) { let _ = owner.derived.finite_raw(Rc::clone(&owner), Some(plan)); }
    const _: () = { let _ = forge; };
}
"#,
        "E0061",
        "KernelPermit",
        "argument",
    ),
};
pub(super) static WITH_RAW_ORDER: Acquire = Acquire {
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
pub(super) static EVALUATE_NATIVE: Acquire = Acquire {
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
pub(super) static PREPARE_GRAPH: Acquire = Acquire {
    positive: r#"
mod census_prepare_graph {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::prepare_graph; };
}
"#,
    negative: refuses(
        r#"
mod census_prepare_graph_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn peek(owner: &DerivedOwner) { let _ = &owner.backend; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "backend",
        "private",
    ),
};
pub(super) static ACQUIRE_GRAPH: Acquire = Acquire {
    positive: r#"
mod census_acquire_graph {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::acquire_graph; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_graph_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    use crate::domain::{derived_views::types::DerivedContract, table_model::types::Rows};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, rows: &Rows<'_>) { let _ = owner.acquire_graph(plan, rows, None); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphInput",
        "mismatched",
    ),
};
pub(super) static ACQUIRE_GRAPH_DOMAIN: Acquire = Acquire {
    positive: r#"
mod census_acquire_graph_domain {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::acquire_graph_domain; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_graph_domain_denied {
    use crate::application::file_dag::{derived::owner::{DerivedOwner, graphs::types::handles::service::DomainHandle}, workspace::GraphInput};
    use crate::domain::derived_views::types::DerivedContract;
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, domain: &DomainHandle, input: &GraphInput<'_>) { let _ = owner.acquire_graph_domain(plan, domain, input, None); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "DomainHandle",
    ),
};
pub(super) static GRAPH_ROW: Acquire = Acquire {
    positive: r#"
mod census_graph_row {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::graph_row; };
}
"#,
    negative: refuses(
        r#"
mod census_graph_row_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::handles::service::GraphHandle};
    use ryo_relations::domain::relations::types::Value;
    fn bypass(owner: &DerivedOwner, graph: &mut GraphHandle, row: &[Value]) { let _ = owner.graph_row(graph, row); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "CachedGraph",
        "GraphHandle",
    ),
};
pub(super) static DEMAND_ANSWERS: Acquire = Acquire {
    positive: r#"
mod census_demand_answers {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::roots::value::DemandAnswers};
    fn demand<'a>(owner: &'a DerivedOwner, cache: Option<&'a mut CacheBackend>) -> DemandAnswers<'a> { DemandAnswers::new(owner, cache) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_demand_answers_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::roots::value::DemandAnswers};
    fn forge(owner: &DerivedOwner) { let _ = DemandAnswers { owner, cache: None }; }
    const _: () = { let _ = forge; };
}
"#,
        "E0451",
        "DemandAnswers",
        "private",
    ),
};
pub(super) static REACH: Acquire = Acquire {
    positive: r#"
mod census_reach {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::reach; };
}
"#,
    negative: refuses(
        r#"
mod census_reach_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::roots::value::DemandAnswers};
    use crate::domain::stored_graph::types::GraphWalk;
    use ryo_derived::domain::contract::types::CompactGraph;
    use ryo_relations::domain::relations::types::{Integer, Value};
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, walk: &mut GraphWalk) { let _ = DemandAnswers::new(owner, None).reach(graph, &[], Value::Number(Integer(1)), walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
pub(super) static AXIS: Acquire = Acquire {
    positive: r#"
mod census_axis {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::axis; };
}
"#,
    negative: refuses(
        r#"
mod census_axis_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::roots::value::DemandAnswers};
    use crate::domain::stored_graph::types::{GraphWalk, Orientation};
    use ryo_derived::domain::contract::types::CompactGraph;
    use ryo_values::domain::plan_parts::types::Direction;
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, walk: &mut GraphWalk) { let _ = DemandAnswers::new(owner, None).axis(graph, 0, Orientation::Forward, Direction::Child, walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
pub(super) static NEAREST: Acquire = Acquire {
    positive: r#"
mod census_nearest {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::nearest; };
}
"#,
    negative: refuses(
        r#"
mod census_nearest_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::{handles::service::DomainHandle, roots::value::DemandAnswers}};
    use crate::domain::stored_graph::types::{GraphWalk, Orientation};
    use ryo_derived::domain::contract::types::CompactGraph;
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, domain: &DomainHandle, walk: &mut GraphWalk) { let _ = DemandAnswers::new(owner, None).nearest(graph, domain, 0, Orientation::Forward, &mut Vec::new(), walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
pub(super) static SELECTOR_PLAN: Acquire = Acquire {
    positive: r#"
mod census_selector_plan {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError, stored_selection::types::Selector};
    use std::sync::Arc;
    fn plan(owner: &DerivedOwner, program: &Arc<Selector>) -> Result<Option<Arc<DerivedContract>>, OperatingError> { owner.selector_plan(program) }
    const _: () = { let _ = plan; };
}
"#,
    negative: refuses(
        r#"
mod census_selector_plan_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn peek(owner: &DerivedOwner) { let _ = &owner.backend; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "backend",
        "private",
    ),
};
pub(super) static SELECT: Acquire = Acquire {
    positive: r#"
mod census_select {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::select; };
}
"#,
    negative: refuses(
        r#"
mod census_select_denied {
    const _: () = { let _ = crate::application::file_dag::derived::owner::selection::select; };
}
"#,
        "E0603",
        "select",
        "private",
    ),
};
pub(super) static PREPARE_PROGRAM_FILE: Acquire = Acquire {
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
pub(super) static PREPARE_VIEWS: Acquire = Acquire {
    positive: r#"
mod census_prepare_views {
    use crate::application::file_dag::derived::owner::selection::preparation::service::prepare_views_with_schemas;
    use crate::domain::{operating::errors::OperatingError, stored_selection::types::{PreparedSelectionViews, Selector}};
    use ryo_relations::domain::relations::types::{CellKind, Relation};
    use std::sync::Arc;
    fn prepare<'s>(selectors: Vec<Arc<Selector>>, resolver: &dyn Fn(Relation) -> &'s [CellKind], port: Arc<dyn crate::outbound::derived::interface::DerivedPort>) -> Result<Arc<PreparedSelectionViews>, OperatingError> { prepare_views_with_schemas(selectors, resolver, port) }
    const _: () = { let _ = prepare; };
}
"#,
    negative: refuses(
        r#"
mod census_prepare_views_denied {
    const _: () = { let _ = crate::application::file_dag::derived::owner::selection::preparation::service::activate_program; };
}
"#,
        "E0603",
        "activate_program",
        "private",
    ),
};

/// Forgery of each authority a consumer may hold, as a struct update, a private field or a
/// private constructor.
pub(super) static AUTHORITY_ACQUIRED: Negative = refuses(
    r#"
mod census_authority_acquired {
    use crate::application::file_dag::derived::owner::acquisition::{admission::service::AcquiredView as Authority, population::types::Construction};
    fn forge(value: Authority) -> Authority { Authority { construction: Construction::Reused, ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "construction",
    "private",
);
pub(super) static AUTHORITY_DEPENDENCY: Negative = refuses(
    r#"
mod census_authority_dependency {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::ResourceDependency as Authority;
    fn forge(value: Authority) -> Authority { Authority { backings: std::sync::Arc::from(value.backings()), ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "backings",
    "private",
);
pub(super) static AUTHORITY_GRAPH: Negative = refuses(
    r#"
mod census_authority_graph {
    use crate::application::file_dag::derived::owner::graphs::types::handles::{service::GraphHandle as Authority, types::GraphSharing};
    fn forge(value: Authority) -> Authority { Authority { sharing: GraphSharing::Shared, ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "sharing",
    "private",
);
pub(super) static AUTHORITY_DOMAIN: Negative = refuses(
    r#"
mod census_authority_domain {
    use crate::application::file_dag::derived::owner::graphs::types::handles::service::DomainHandle as Authority;
    fn forge(left: &mut Authority, right: &mut Authority) { std::mem::swap(&mut left.parent, &mut right.parent); }
    const _: () = { let _ = forge; };
}
"#,
    "E0616",
    "parent",
    "private",
);
pub(super) static AUTHORITY_ANSWER: Negative = refuses(
    r#"
mod census_authority_answer {
    use crate::application::file_dag::derived::owner::graphs::types::handles::service::RootAnswer;
    const _: () = { let _ = RootAnswer::new; };
}
"#,
    "E0624",
    "new",
    "private",
);
pub(super) static AUTHORITY_CACHED: Negative = refuses(
    r#"
mod census_authority_cached {
    use crate::application::file_dag::cached_graph::ReachGraph as Authority;
    use crate::domain::stored_graph::types::GraphWalk;
    fn forge(value: Authority) -> Authority { Authority { walk: GraphWalk::default(), ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "walk",
    "private",
);

/// Reads of leased views and handles: the accessor is nameable from a consumer.
pub(super) fn read(path: &str, owner_type: &str, method: &str) -> String {
    format!(
        "mod census_read_{owner_type}_{method} {{ use {path}::{owner_type}; const _: () = {{ let _ = {owner_type}::{method}; }}; }}\n"
    )
}
