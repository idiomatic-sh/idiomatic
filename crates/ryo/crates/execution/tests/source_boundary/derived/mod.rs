//! Same-crate controls target the actual derived owner and its actual private issuers.
use super::fixtures::{Expectation, Probe};
pub(super) const ROOT: &str = "src/application/file_dag/derived.rs";
pub(super) const MODULE: &str = "src/application/file_dag/derived/owner/acquisition/mod.rs";
pub(super) const ISSUER: &str =
    "src/application/file_dag/derived/owner/acquisition/admission/service.rs";
const AUTHORIZED: &str = r#"
mod derived_authorized {
    use crate::application::file_dag::relations::{RelationOwner, AdmittedInput};
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
pub(super) fn controls() -> Vec<Probe> {
    vec![
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
