//! Same-crate controls target the actual derived owner and its actual private issuers.
use super::fixtures::{Expectation, Probe};
pub(super) const ROOT: &str = "src/application/file_dag/derived.rs";
pub(super) const GRAPH_ISSUER: &str =
    "src/application/file_dag/derived/owner/graphs/types/handles/service.rs";
pub(super) const MODULE: &str = "src/application/file_dag/derived/owner/acquisition/mod.rs";
pub(super) const ISSUER: &str =
    "src/application/file_dag/derived/owner/acquisition/admission/service.rs";
const AUTHORIZED: &str = r#"
mod derived_authorized {
    use crate::application::file_dag::relations::{RelationOwner, admission::service::AdmittedInput};
    use crate::application::file_dag::program::runtime::service::KernelPermit;
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use crate::domain::operating::errors::OperatingError;
    use std::{rc::Rc, sync::Arc};
    fn acquire<'a>(owner: &'a RelationOwner, plan: &'a Arc<DerivedContract>, proof: &'a AdmittedInput<'a>, scratch: &'a mut DerivedScratch) -> Result<(), OperatingError> {
        let admitted = owner.derived.admit_view(plan, owner, &[proof])?;
        let _ = owner.derived.acquire_stored(plan, &admitted, Default::default(), DerivedInputs::Rows(&[proof.rows()]), scratch, None)?;
        Ok(())
    }
    const _: for<'a> fn(&'a RelationOwner, &'a Arc<DerivedContract>, &'a AdmittedInput<'a>, &'a mut DerivedScratch) -> Result<(), OperatingError> = acquire;
    fn raw<'a>(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>, permit: &'a KernelPermit<'a>) -> Result<(), OperatingError> {
        let _ = owner.derived.finite_raw(Rc::clone(&owner), Some(plan), permit)?;
        Ok(())
    }
    const _: for<'a> fn(Rc<RelationOwner>, Arc<DerivedContract>, &'a KernelPermit<'a>) -> Result<(), OperatingError> = raw;
}
"#;
const PRIVATE_OWNER: &str = r#"
mod derived_private_owner {
    use super::AdmittedViewInputs as Authority;
    fn retarget<'a>(left: &mut Authority<'a>, right: &mut Authority<'a>) {
        std::mem::swap(&mut left.sources, &mut right.sources);
    }
    const _: for<'a> fn(&mut Authority<'a>, &mut Authority<'a>) = retarget;
}
"#;
const ADMISSION_ALIAS: &str = r#"
mod future_consumer {
    use super::admission::service::AdmittedViewInputs as Authority;
    fn retarget<'a>(left: &mut Authority<'a>, right: &mut Authority<'a>) {
        std::mem::swap(&mut left.sources, &mut right.sources);
    }
    const _: for<'a> fn(&mut Authority<'a>, &mut Authority<'a>) = retarget;
}
"#;
const ACQUIRED_ALIAS: &str = r#"
mod acquired_alias {
    use super::admission::service::AcquiredView as Authority;
    fn forge(value: Authority) -> Authority { Authority { built: false, ..value } }
    const _: fn(Authority) -> Authority = forge;
}
"#;
const ACQUIRED_SELF: &str = r#"
mod acquired_self {
    use super::admission::service::AcquiredView as Authority;
    impl Authority { fn denied_record_update(value: Self) -> Self { Self { built: false, ..value } } }
    const _: fn(Authority) -> Authority = Authority::denied_record_update;
}
"#;
const STORAGE_CONSTRUCTOR: &str = r#"
mod storage_constructor {
    use super::semantics::service::DerivedStorage as Authority;
    use crate::outbound::derived::interface::DerivedPort;
    use ryo_derived::domain::contract::types::{PreparedDerived, DerivedPayload};
    use std::sync::Arc;
    fn forge(port: Arc<dyn DerivedPort>, plan: Arc<PreparedDerived>, payload: DerivedPayload) { let _ = Authority::new(port, plan, payload, None); }
    const _: fn(Arc<dyn DerivedPort>, Arc<PreparedDerived>, DerivedPayload) = forge;
}
"#;
const UNPERMITTED_RAW: &str = r#"
mod unpermitted_raw {
    use crate::application::file_dag::relations::RelationOwner;
    use crate::domain::derived_views::types::DerivedContract;
    use std::{rc::Rc, sync::Arc};
    fn forge(owner: Rc<RelationOwner>, plan: Arc<DerivedContract>) { let _ = owner.derived.finite_raw(Rc::clone(&owner), Some(plan)); }
    const _: fn(Rc<RelationOwner>, Arc<DerivedContract>) = forge;
}
"#;
const REMOVED_PORT_GETTER: &str = r#"
mod removed_port_getter {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn escape(owner: &DerivedOwner) { let _ = owner.semantic_port(); }
    const _: fn(&DerivedOwner) = escape;
}
"#;
const RAW_BRIDGE_SIBLING: &str = r#"
mod future_raw_consumer {
    use crate::outbound::derived::interface::DerivedPort;
    use crate::domain::derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch};
    use std::sync::Arc;
    fn bypass<'a>(port: Arc<dyn DerivedPort>, plan: &'a DerivedContract, inputs: DerivedInputs<'a, 'a>, scratch: &'a mut DerivedScratch) {
        let _ = crate::application::file_dag::derived::owner::acquisition::semantics::service::build(port, plan, inputs, scratch, None);
    }
    const _: for<'a> fn(Arc<dyn DerivedPort>, &'a DerivedContract, DerivedInputs<'a, 'a>, &'a mut DerivedScratch) = bypass;
}
"#;
const SOURCE_RETARGET: &str = r#"
mod acquired_source_retarget {
    use super::admission::service::AcquiredView as Authority;
    fn retarget(left: &mut Authority, right: &mut Authority) { std::mem::swap(&mut left.origin, &mut right.origin); }
    const _: fn(&mut Authority, &mut Authority) = retarget;
}
"#;
const OWNED_STORAGE_ESCAPE: &str = r#"
mod owned_storage_escape { use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView; fn escape(view: &AcquiredView) { let _ = std::sync::Arc::clone(view.storage()); } const _: fn(&AcquiredView) = escape; }
"#;
const RAW_PAYLOAD_ESCAPE: &str = r#"
mod raw_payload_escape { use crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage; fn escape(storage: &DerivedStorage) { let _ = storage.payload(); } const _: fn(&DerivedStorage) = escape; }
"#;
const RAW_VIEW_ESCAPE: &str = r#"
mod raw_view_escape { use crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage; fn escape(storage: &DerivedStorage) { let _ = storage.view(); } const _: fn(&DerivedStorage) = escape; }
"#;
const GRAPH_ALLOCATION_ESCAPE: &str = r#"
mod graph_allocation_escape { use crate::application::file_dag::derived::owner::graphs::types::handles::service::GraphHandle; use ryo_derived::domain::contract::types::CompactGraph; fn escape(value: &GraphHandle) { let _: std::sync::Arc<CompactGraph> = std::sync::Arc::clone(value.graph()); } const _: fn(&GraphHandle) = escape; }
"#;
const DOMAIN_ALLOCATION_ESCAPE: &str = r#"
mod domain_allocation_escape { use crate::application::file_dag::derived::owner::graphs::types::handles::service::DomainHandle; fn escape(value: &DomainHandle) { let _: std::sync::Arc<Vec<bool>> = std::sync::Arc::clone(value.members()); } const _: fn(&DomainHandle) = escape; }
"#;
const GRAPH_RECORD_FORGERY: &str = r#"
mod graph_record_forgery { use crate::application::file_dag::derived::owner::graphs::types::handles::service::GraphHandle as Authority; fn forge(value: Authority) -> Authority { Authority { shared: true, ..value } } const _: fn(Authority) -> Authority = forge; }
"#;
const UNADMITTED_GRAPH_DEMAND: &str = r#"
mod unadmitted_graph_demand { use crate::application::file_dag::derived::owner::{DerivedOwner, graphs::types::roots::value::DemandAnswers}; use crate::domain::stored_graph::types::GraphWalk; use ryo_derived::domain::contract::types::CompactGraph; use ryo_relations::domain::relations::types::{Value, Integer}; fn forge(owner: &DerivedOwner, graph: &CompactGraph, walk: &mut GraphWalk) { let _ = DemandAnswers::new(owner, None).reach(graph, &[], Value::Number(Integer(1)), walk); } const _: fn(&DerivedOwner, &CompactGraph, &mut GraphWalk) = forge; }
"#;
const PRIVATE_RETAINED_STORAGE: &str = r#"
mod private_retained_storage { use crate::application::file_dag::derived::owner::cache::types::CacheBackend; fn escape(cache: &CacheBackend) { let _ = &cache.graphs; } const _: fn(&CacheBackend) = escape; }
"#;
const BORROWED_LEASED_READ: &str = r#"
mod borrowed_leased_read { use crate::application::file_dag::derived::owner::graphs::types::handles::service::{GraphHandle, DomainHandle}; fn read(graph: &GraphHandle, domain: &DomainHandle) -> usize { graph.graph().vertices().len() + domain.members().len() } const _: fn(&GraphHandle, &DomainHandle) -> usize = read; }
"#;
const OWNED_GRAPH_PRIVATE_FIELDS: &str = r#"
mod owned_graph_private_fields { use super::GraphHandle as Authority; fn policy(value: &mut Authority) { value.shared = false; } const _: fn(&mut Authority) = policy; }
"#;
const GRAPH_SELF_FORGERY: &str = r#"
mod graph_self_forgery { use crate::application::file_dag::derived::owner::graphs::types::handles::service::GraphHandle as Authority; impl Authority { fn denied_graph(value: Self) -> Self { Self { shared: false, ..value } } } const _: fn(Authority) -> Authority = Authority::denied_graph; }
"#;
const DOMAIN_ALIAS_FORGERY: &str = r#"
mod domain_alias_forgery { use crate::application::file_dag::derived::owner::graphs::types::handles::service::DomainHandle as Authority; fn forge(left: &mut Authority, right: &mut Authority) { std::mem::swap(&mut left.parent, &mut right.parent); } const _: fn(&mut Authority, &mut Authority) = forge; }
"#;
const GRAPH_RAW_GETTER: &str = r#"
mod graph_raw_getter { use crate::application::file_dag::derived::owner::graphs::types::handles::service::GraphHandle; fn escape(value: &GraphHandle) { let _ = std::sync::Arc::clone(value.raw_graph()); } const _: fn(&GraphHandle) = escape; }
"#;
const DOMAIN_RAW_GETTER: &str = r#"
mod domain_raw_getter { use crate::application::file_dag::derived::owner::graphs::types::handles::service::DomainHandle; fn escape(value: &DomainHandle) { let _ = std::sync::Arc::clone(value.raw_domain()); } const _: fn(&DomainHandle) = escape; }
"#;
pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            owner: ROOT,
            source: GRAPH_SELF_FORGERY.into(),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "shared",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: DOMAIN_ALIAS_FORGERY.into(),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "parent",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: GRAPH_RAW_GETTER.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "raw_graph",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: DOMAIN_RAW_GETTER.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "raw_domain",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: OWNED_STORAGE_ESCAPE.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "storage",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: RAW_PAYLOAD_ESCAPE.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "payload",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: RAW_VIEW_ESCAPE.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "view",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: GRAPH_ALLOCATION_ESCAPE.into(),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "Arc",
                second: "CompactGraph",
            },
        },
        Probe {
            owner: ROOT,
            source: DOMAIN_ALLOCATION_ESCAPE.into(),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "Arc",
                second: "bool",
            },
        },
        Probe {
            owner: ROOT,
            source: GRAPH_RECORD_FORGERY.into(),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "shared",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: UNADMITTED_GRAPH_DEMAND.into(),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "GraphHandle",
                second: "CompactGraph",
            },
        },
        Probe {
            owner: ROOT,
            source: PRIVATE_RETAINED_STORAGE.into(),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "graphs",
                second: "private",
            },
        },
        Probe {
            owner: ROOT,
            source: BORROWED_LEASED_READ.into(),
            expected: Expectation::Accept,
        },
        Probe {
            owner: GRAPH_ISSUER,
            source: OWNED_GRAPH_PRIVATE_FIELDS.into(),
            expected: Expectation::Accept,
        },
        Probe {
            owner: ROOT,
            source: RAW_BRIDGE_SIBLING.into(),
            expected: Expectation::Refuse {
                code: "E0603",
                first: "build",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: SOURCE_RETARGET.into(),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "origin",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: AUTHORIZED.into(),
            expected: Expectation::Accept,
        },
        Probe {
            owner: ISSUER,
            source: PRIVATE_OWNER.into(),
            expected: Expectation::Accept,
        },
        Probe {
            owner: MODULE,
            source: ADMISSION_ALIAS.into(),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "sources",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: ACQUIRED_ALIAS.into(),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "built",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: ACQUIRED_SELF.into(),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "built",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: STORAGE_CONSTRUCTOR.into(),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "new",
                second: "private",
            },
        },
        Probe {
            owner: MODULE,
            source: UNPERMITTED_RAW.into(),
            expected: Expectation::Refuse {
                code: "E0061",
                first: "KernelPermit",
                second: "argument",
            },
        },
        Probe {
            owner: MODULE,
            source: REMOVED_PORT_GETTER.into(),
            expected: Expectation::Refuse {
                code: "E0599",
                first: "semantic_port",
                second: "DerivedOwner",
            },
        },
    ]
}
