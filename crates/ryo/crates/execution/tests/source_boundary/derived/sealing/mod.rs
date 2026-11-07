//! Sealed authority controls: contract forgery, dependency forgery, backend mutation, build escape.
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use super::super::fixtures::{Compilation, Expectation, Probe};

pub(super) const TYPES: &str = "src/application/file_dag/derived/owner/types.rs";
pub(super) const CACHE: &str = "src/application/file_dag/derived/owner/cache/types.rs";
/// Every non-test field of the backend, read from its declaration, so a new field is probed
/// individually and a public field is refused by its own probe.
fn cache_fields() -> Vec<String> {
    let source =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CACHE))
            .unwrap();
    let start = source
        .find("struct CacheBackend {")
        .expect("CacheBackend is declared in its source");
    let lines: Vec<&str> = source[start..].lines().skip(1).collect();
    let mut fields = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "}" {
            break;
        }
        if !(trimmed.starts_with("pub(") || trimmed.starts_with("pub "))
            || !line.starts_with("    pub")
        {
            continue;
        }
        if at > 0 && lines[at - 1].trim() == "#[cfg(test)]" {
            continue;
        }
        let after = trimmed.split_once(") ").map_or(trimmed, |(_, rest)| rest);
        let name = after.split(':').next().unwrap().trim();
        fields.push(name.to_owned());
    }
    assert!(
        fields.len() >= 13,
        "the backend declares at least the 13 fields known at the fold: {fields:?}"
    );
    fields
}
const CONTRACT_UPDATE: &str = r#"
mod contract_update { use crate::domain::derived_views::types::{contract::value::DerivedContract as Authority, IdentityHash}; fn forge(value: Authority) -> Authority { Authority { identity_hash: IdentityHash(0), ..value } } const _: fn(Authority) -> Authority = forge; }
"#;
const CONTRACT_READ: &str = r#"
mod contract_read { use crate::domain::derived_views::types::{contract::value::DerivedContract as Authority, IdentityHash}; fn peek(value: &Authority) -> IdentityHash { value.identity_hash } const _: fn(&Authority) -> IdentityHash = peek; }
"#;
const CONTRACT_PREPARED: &str = r#"
mod contract_prepared {
    use crate::domain::{derived_views::types::{contract::value::DerivedContract, DerivedRequest}, operating::errors::OperatingError, operation_schema::types::InputShape};
    use crate::outbound::derived::interface::DerivedPort;
    use std::sync::Arc;
    fn authorized(port: Arc<dyn DerivedPort>, request: &DerivedRequest, schemas: &[InputShape<'_>]) -> Result<DerivedContract, OperatingError> {
        crate::application::file_dag::derived::prepare::view(port, request, schemas)
    }
    const _: fn(Arc<dyn DerivedPort>, &DerivedRequest, &[InputShape<'_>]) -> Result<DerivedContract, OperatingError> = authorized;
}
"#;
const CACHE_MUTATOR: &str = r#"
mod cache_mutator { use crate::application::file_dag::derived::owner::cache::types::CacheBackend; fn mutate(cache: &mut CacheBackend) { cache.observe_graph_walk(crate::domain::stored_graph::types::ExpandedVertices(1), crate::domain::stored_graph::types::ExaminedVertices(1)); } const _: fn(&mut CacheBackend) = mutate; }
"#;

const PORT_BUILD: &str = r#"
mod port_build { use crate::application::file_dag::derived::owner::types::DerivedAccess; fn escape(access: &DerivedAccess) { let _ = access.build(); } const _: fn(&DerivedAccess) = escape; }
"#;
const PORT_FIELD: &str = r#"
mod port_field { use crate::application::file_dag::derived::owner::types::DerivedAccess; fn escape(access: &DerivedAccess) { let _ = &access.port; } const _: fn(&DerivedAccess) = escape; }
"#;
const PORT_ACCESSOR: &str = r#"
mod port_accessor { use crate::application::file_dag::derived::owner::types::DerivedAccess; fn escape(access: &DerivedAccess) { let _ = access.port(); } const _: fn(&DerivedAccess) = escape; }
"#;
const ACCESS_PREPARE: &str = r#"
mod access_prepare {
    use crate::application::file_dag::derived::owner::types::DerivedAccess;
    use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
    use ryo_derived::domain::request::types::DerivedRequest;
    use std::sync::Arc;
    fn prepare(access: &DerivedAccess, request: DerivedRequest) -> Arc<PreparedDerived> { access.prepare(request, Vec::new()) }
    const _: fn(&DerivedAccess, DerivedRequest) -> Arc<PreparedDerived> = prepare;
}
"#;
const INBOUND_BUILD: &str = r#"
mod inbound_build {
    use super::PrepareUseCaseImpl;
    fn escape(use_case: &PrepareUseCaseImpl) { let _ = use_case.derived.build(); }
    const _: fn(&PrepareUseCaseImpl) = escape;
}
"#;
const INBOUND_PORT: &str = r#"
mod inbound_port {
    use super::PrepareUseCaseImpl;
    use crate::outbound::derived::interface::DerivedPort;
    use std::sync::Arc;
    fn escape(use_case: &PrepareUseCaseImpl) -> &Arc<dyn DerivedPort> { &use_case.derived }
    const _: fn(&PrepareUseCaseImpl) -> &Arc<dyn DerivedPort> = escape;
}
"#;
const INBOUND_HOLDS: &str = r#"
mod inbound_holds {
    use super::PrepareUseCaseImpl;
    use crate::application::file_dag::derived::owner::types::DerivedAccess;
    fn hold(use_case: &PrepareUseCaseImpl) -> DerivedAccess { use_case.derived.clone() }
    const _: fn(&PrepareUseCaseImpl) -> DerivedAccess = hold;
}
"#;
const PORT_OWNER: &str = r#"
mod port_owner { use super::DerivedAccess; fn own(access: &DerivedAccess) { let _ = access.port(); } const _: fn(&DerivedAccess) = own; }
"#;

fn refuse(
    owner: &'static str,
    source: String,
    code: &'static str,
    first: &'static str,
    second: &'static str,
) -> Probe {
    Probe {
        compilation: Compilation::Library,
        owner,
        source,
        expected: Expectation::Refuse {
            code,
            first,
            second,
        },
    }
}

pub(super) fn controls(root: &'static str) -> Vec<Probe> {
    let fields = cache_fields();
    let field_reads = fields
        .iter()
        .map(|field| format!("let _ = &cache.{field};"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut probes = vec![
        refuse(root, CONTRACT_UPDATE.into(), "E0451", "identity_hash", "private"),
        refuse(root, CONTRACT_READ.into(), "E0616", "identity_hash", "private"),
        Probe { compilation: Compilation::Library, owner: root, source: CONTRACT_PREPARED.into(), expected: Expectation::Accept },
        refuse(root, CACHE_MUTATOR.into(), "E0624", "observe_graph_walk", "private"),
        Probe {
            compilation: Compilation::Library,
            owner: CACHE,
            source: format!(
                "mod cache_owner {{ use super::CacheBackend; fn own(cache: &mut CacheBackend) {{ {field_reads} cache.observe_graph_walk(crate::domain::stored_graph::types::ExpandedVertices(1), crate::domain::stored_graph::types::ExaminedVertices(1)); }} const _: fn(&mut CacheBackend) = own; }}"
            ),
            expected: Expectation::Accept,
        },
        // The builder port: a file_dag sibling can prepare through the capability, and can
        // neither call build, name the accessor nor read the port; the owner reads it.
        refuse(root, PORT_BUILD.into(), "E0599", "build", "DerivedAccess"),
        refuse(root, PORT_ACCESSOR.into(), "E0624", "port", "private"),
        refuse(root, PORT_FIELD.into(), "E0616", "port", "private"),
        Probe { compilation: Compilation::Library, owner: root, source: ACCESS_PREPARE.into(), expected: Expectation::Accept },
        Probe { compilation: Compilation::Library, owner: TYPES, source: PORT_OWNER.into(), expected: Expectation::Accept },
        // The inbound layer holds the capability, not the port: it cannot call build through
        // the field, cannot recover the port type from it, and can hold and clone the capability.
        refuse(super::INBOUND, INBOUND_BUILD.into(), "E0599", "build", "DerivedAccess"),
        refuse(super::INBOUND, INBOUND_PORT.into(), "E0308", "DerivedPort", "DerivedAccess"),
        Probe { compilation: Compilation::Library, owner: super::INBOUND, source: INBOUND_HOLDS.into(), expected: Expectation::Accept },
    ];
    for field in &fields {
        probes.push(refuse(
            root,
            format!("mod cache_field_{field} {{ use crate::application::file_dag::derived::owner::cache::types::CacheBackend; fn escape(cache: &CacheBackend) {{ let _ = &cache.{field}; }} const _: fn(&CacheBackend) = escape; }}"),
            "E0616",
            "CacheBackend",
            "private",
        ));
    }
    probes
}
