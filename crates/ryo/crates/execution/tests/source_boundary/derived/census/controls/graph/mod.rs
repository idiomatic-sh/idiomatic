//! Controls of the graph entries of the derived owner.
use super::{Acquire, refuses};

pub(in super::super) static PREPARE_GRAPH: Acquire = Acquire {
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
pub(in super::super) static ACQUIRE_GRAPH: Acquire = Acquire {
    positive: r#"
mod census_acquire_graph {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::acquire_graph; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_graph_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend};
    use crate::domain::{derived_views::types::DerivedContract, table_model::types::Rows};
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, rows: &Rows<'_>, cache: &mut CacheBackend) { let _ = owner.acquire_graph(plan, rows, cache); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphInput",
        "mismatched",
    ),
};
pub(in super::super) static ACQUIRE_GRAPH_DOMAIN: Acquire = Acquire {
    positive: r#"
mod census_acquire_graph_domain {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::acquire_graph_domain; };
}
"#,
    negative: refuses(
        r#"
mod census_acquire_graph_domain_denied {
    use crate::application::file_dag::{derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::handles::service::DomainHandle}, workspace::GraphInput};
    use crate::domain::derived_views::types::DerivedContract;
    use std::sync::Arc;
    fn bypass(owner: &DerivedOwner, plan: &Arc<DerivedContract>, domain: &DomainHandle, input: &GraphInput<'_>, cache: &mut CacheBackend) { let _ = owner.acquire_graph_domain(plan, domain, input, cache); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "DomainHandle",
    ),
};
pub(in super::super) static GRAPH_ROW: Acquire = Acquire {
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
pub(in super::super) static DEMAND_ANSWERS: Acquire = Acquire {
    positive: r#"
mod census_demand_answers {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::roots::value::DemandAnswers};
    fn demand<'a>(owner: &'a DerivedOwner, cache: &'a mut CacheBackend) -> DemandAnswers<'a> { DemandAnswers::new(owner, cache) }
    const _: () = { let _ = demand; };
}
"#,
    negative: refuses(
        r#"
mod census_demand_answers_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::roots::value::DemandAnswers};
    fn forge<'a>(owner: &'a DerivedOwner, cache: &'a mut CacheBackend) { let _ = DemandAnswers { owner, cache }; }
    const _: () = { let _ = forge; };
}
"#,
        "E0451",
        "DemandAnswers",
        "private",
    ),
};
pub(in super::super) static REACH: Acquire = Acquire {
    positive: r#"
mod census_reach {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::reach; };
}
"#,
    negative: refuses(
        r#"
mod census_reach_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::roots::value::DemandAnswers};
    use crate::domain::stored_graph::types::GraphWalk;
    use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
    use ryo_relations::domain::relations::types::{Integer, Value};
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, walk: &mut GraphWalk, cache: &mut CacheBackend) { let _ = DemandAnswers::new(owner, cache).reach(graph, &[], Value::Number(Integer(1)), walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
pub(in super::super) static AXIS: Acquire = Acquire {
    positive: r#"
mod census_axis {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::axis; };
}
"#,
    negative: refuses(
        r#"
mod census_axis_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::roots::value::DemandAnswers};
    use crate::domain::stored_graph::types::{GraphWalk, Orientation};
    use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
    use ryo_values::domain::plan_parts::types::Direction;
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, walk: &mut GraphWalk, cache: &mut CacheBackend) { let _ = DemandAnswers::new(owner, cache).axis(graph, 0, Orientation::Forward, Direction::Child, walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
pub(in super::super) static NEAREST: Acquire = Acquire {
    positive: r#"
mod census_nearest {
    use crate::application::file_dag::derived::owner::graphs::types::roots::value::DemandAnswers;
    const _: () = { let _ = DemandAnswers::nearest; };
}
"#,
    negative: refuses(
        r#"
mod census_nearest_denied {
    use crate::application::file_dag::derived::owner::{DerivedOwner, cache::types::CacheBackend, graphs::types::{handles::service::DomainHandle, roots::value::DemandAnswers}};
    use crate::domain::stored_graph::types::{GraphWalk, Orientation};
    use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
    fn bypass(owner: &DerivedOwner, graph: &CompactGraph, domain: &DomainHandle, walk: &mut GraphWalk, cache: &mut CacheBackend) { let _ = DemandAnswers::new(owner, cache).nearest(graph, domain, 0, Orientation::Forward, &mut Vec::new(), walk); }
    const _: () = { let _ = bypass; };
}
"#,
        "E0308",
        "GraphHandle",
        "CompactGraph",
    ),
};
