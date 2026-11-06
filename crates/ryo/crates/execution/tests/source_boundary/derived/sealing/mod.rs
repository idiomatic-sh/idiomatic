//! Sealed authority controls: contract forgery, dependency forgery, backend mutation, build escape.
use super::super::fixtures::{Compilation, Expectation, Probe};

pub(super) const CACHE: &str = "src/application/file_dag/derived/owner/cache/types.rs";
/// Every non-test field of the backend, read from its declaration, so a new field is probed
/// individually and a public field is refused by its own probe.
fn cache_fields() -> Vec<String> {
    let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CACHE)).unwrap();
    let start = source.find("struct CacheBackend {").expect("CacheBackend is declared in its source");
    let lines: Vec<&str> = source[start..].lines().skip(1).collect();
    let mut fields = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "}" {
            break;
        }
        if !(trimmed.starts_with("pub(") || trimmed.starts_with("pub ")) || !line.starts_with("    pub") {
            continue;
        }
        if at > 0 && lines[at - 1].trim() == "#[cfg(test)]" {
            continue;
        }
        let after = trimmed.split_once(") ").map_or(trimmed, |(_, rest)| rest);
        let name = after.split(':').next().unwrap().trim();
        fields.push(name.to_owned());
    }
    assert!(fields.len() >= 21, "the backend declares at least the 21 fields known at the review: {fields:?}");
    fields
}
const CONTRACT_UPDATE: &str = r#"
mod contract_update { use crate::domain::derived_views::types::DerivedContract as Authority; fn forge(value: Authority) -> Authority { Authority { identity_hash: 0, ..value } } const _: fn(Authority) -> Authority = forge; }
"#;
const CONTRACT_READ: &str = r#"
mod contract_read { use crate::domain::derived_views::types::DerivedContract as Authority; fn peek(value: &Authority) -> u64 { value.identity_hash } const _: fn(&Authority) -> u64 = peek; }
"#;
const CONTRACT_PREPARED: &str = r#"
mod contract_prepared {
    use crate::domain::{derived_views::types::{DerivedContract, ViewRequest}, operating::errors::OperatingError, operation_schema::types::InputShape};
    use crate::outbound::derived::interface::DerivedPort;
    use std::sync::Arc;
    fn authorized(port: Arc<dyn DerivedPort>, request: &ViewRequest, schemas: &[InputShape<'_>]) -> Result<DerivedContract, OperatingError> {
        crate::application::file_dag::derived::prepare::view(port, request, schemas)
    }
    const _: fn(Arc<dyn DerivedPort>, &ViewRequest, &[InputShape<'_>]) -> Result<DerivedContract, OperatingError> = authorized;
}
"#;
const DEPENDENCY_FORGERY: &str = r#"
mod dependency_forgery {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::ResourceDependency as Authority;
    fn forge(value: Authority) -> Authority { Authority { backings: std::sync::Arc::from(value.backings()), ..value } }
    const _: fn(Authority) -> Authority = forge;
}
"#;
const DEPENDENCY_AUTHORIZED: &str = r#"
mod dependency_authorized {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::{AcquiredView, ResourceDependency};
    fn lease(view: &AcquiredView) -> ResourceDependency { view.dependency() }
    const _: fn(&AcquiredView) -> ResourceDependency = lease;
}
"#;
const CACHE_MUTATOR: &str = r#"
mod cache_mutator { use crate::application::file_dag::derived::owner::cache::types::CacheBackend; fn mutate(cache: &mut CacheBackend) { cache.observe_graph_walk(crate::application::file_dag::derived::owner::cache::types::ExpandedVertices(1), crate::application::file_dag::derived::owner::cache::types::ExaminedVertices(1)); } const _: fn(&mut CacheBackend) = mutate; }
"#;
const CACHE_BINDING: &str = r#"
mod cache_binding {
    use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
    use crate::domain::stored_selection::types::PreparedSelectionViews;
    use std::sync::Arc;
    fn mutate(cache: &mut CacheBackend, views: Arc<PreparedSelectionViews>) { cache.bind_request_views(views); }
    const _: fn(&mut CacheBackend, Arc<PreparedSelectionViews>) = mutate;
}
"#;
const SELECTION_BUILD: &str = r#"
mod selection_build {
    use super::selection::service::SelectionSource;
    use crate::domain::{derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch}, operating::errors::OperatingError};
    use std::sync::Arc;
    fn escape(source: &SelectionSource<'_>, plan: &Arc<DerivedContract>, inputs: DerivedInputs<'_, '_>, scratch: &mut DerivedScratch) -> Result<(), OperatingError> {
        source.build(plan, inputs, scratch).map(|_| ())
    }
    const _: fn(&SelectionSource<'_>, &Arc<DerivedContract>, DerivedInputs<'_, '_>, &mut DerivedScratch) -> Result<(), OperatingError> = escape;
}
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

pub(super) fn controls(root: &'static str, module: &'static str) -> Vec<Probe> {
    let fields = cache_fields();
    let field_reads = fields.iter().map(|field| format!("let _ = &cache.{field};")).collect::<Vec<_>>().join(" ");
    let mut probes = vec![
        refuse(root, CONTRACT_UPDATE.into(), "E0451", "identity_hash", "private"),
        refuse(root, CONTRACT_READ.into(), "E0616", "identity_hash", "private"),
        Probe { compilation: Compilation::Library, owner: root, source: CONTRACT_PREPARED.into(), expected: Expectation::Accept },
        refuse(root, DEPENDENCY_FORGERY.into(), "E0451", "backings", "private"),
        Probe { compilation: Compilation::Library, owner: root, source: DEPENDENCY_AUTHORIZED.into(), expected: Expectation::Accept },
        Probe {
            compilation: Compilation::LibraryTests,
            owner: root,
            source: "mod cache_test_field { use crate::application::file_dag::derived::owner::cache::types::CacheBackend; fn escape(cache: &CacheBackend) { let _ = &cache.selector_descriptor; } const _: fn(&CacheBackend) = escape; }".into(),
            expected: Expectation::Refuse { code: "E0616", first: "CacheBackend", second: "private" },
        },
        refuse(root, CACHE_MUTATOR.into(), "E0624", "observe_graph_walk", "private"),
        refuse(root, CACHE_BINDING.into(), "E0624", "bind_request_views", "private"),
        Probe {
            compilation: Compilation::Library,
            owner: CACHE,
            source: format!(
                "mod cache_owner {{ use super::CacheBackend; use crate::domain::stored_selection::types::PreparedSelectionViews; use std::sync::Arc; fn own(cache: &mut CacheBackend, views: Arc<PreparedSelectionViews>) {{ {field_reads} cache.observe_graph_walk(crate::application::file_dag::derived::owner::cache::types::ExpandedVertices(1), crate::application::file_dag::derived::owner::cache::types::ExaminedVertices(1)); cache.bind_request_views(views); }} const _: fn(&mut CacheBackend, Arc<PreparedSelectionViews>) = own; }}"
            ),
            expected: Expectation::Accept,
        },
        refuse(module, SELECTION_BUILD.into(), "E0624", "build", "private"),
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
