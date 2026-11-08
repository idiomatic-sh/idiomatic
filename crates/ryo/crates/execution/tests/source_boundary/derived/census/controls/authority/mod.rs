//! Forgery controls of each authority a consumer may hold.
use super::{Negative, refuses};

/// Forgery of each authority a consumer may hold, as a struct update, a private field or a
/// private constructor.
pub(in super::super) static AUTHORITY_ACQUIRED: Negative = refuses(
    r#"
mod census_authority_acquired {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView as Authority;
    fn forge(value: Authority) -> Authority { Authority { backings: std::sync::Arc::from(Vec::new()), ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "backings",
    "private",
);
pub(in super::super) static AUTHORITY_GRAPH: Negative = refuses(
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
pub(in super::super) static AUTHORITY_DOMAIN: Negative = refuses(
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
pub(in super::super) static AUTHORITY_ANSWER: Negative = refuses(
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
pub(in super::super) static AUTHORITY_ACCESS: Negative = refuses(
    r#"
mod census_authority_access {
    use crate::application::file_dag::derived::owner::types::DerivedAccess;
    fn peek(access: &DerivedAccess) { let _ = access.port(); }
    const _: () = { let _ = peek; };
}
"#,
    "E0624",
    "port",
    "private",
);
pub(in super::super) static AUTHORITY_CACHED: Negative = refuses(
    r#"
mod census_authority_cached {
    use crate::application::file_dag::derived::owner::cached_graph::types::ReachGraph as Authority;
    use ryo_execution_domain::domain::stored_graph::types::GraphWalk;
    fn forge(value: Authority) -> Authority { Authority { walk: GraphWalk::default(), ..value } }
    const _: () = { let _ = forge; };
}
"#,
    "E0451",
    "walk",
    "private",
);
