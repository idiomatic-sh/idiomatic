//! The rows of the entries that acquire a view: each names its positive and negative compile control.
//! Who may call each entry is `idiomatic.ryo/derived-view-acquisition`'s `allowed` list.
use super::super::*;

pub(super) fn row(entry: Entry) -> Row {
    let admission = owner!("acquisition/admission/service/issuance/service.rs");
    let acquisition = owner!("acquisition/service.rs");
    let graph = owner!("graph.rs");
    let roots = owner!("graphs/types/roots/value.rs");
    let setup = owner!("setup.rs");
    match entry {
        Entry::AdmitView => acquires(admission, "admit_view", &c::admission::ADMIT_VIEW),
        Entry::AdmitPartition => acquires(admission, "admit_partition", &c::admission::ADMIT_PARTITION),
        Entry::AcquireStored => acquires(acquisition, "acquire_stored", &c::admission::ACQUIRE_STORED),
        Entry::AcquireStoredWithRoute => acquires(
            acquisition,
            "acquire_stored_with_route",
            &c::admission::ACQUIRE_STORED_WITH_ROUTE,
        ),
        Entry::TryAcquireRows => acquires(acquisition, "try_acquire_rows", &c::admission::TRY_ACQUIRE_ROWS),
        Entry::ScopedBuild => acquires(
            owner!("acquisition/scoped/service.rs"),
            "scoped_build",
            &c::admission::SCOPED_BUILD,
        ),
        Entry::PrepareGraph => acquires(graph, "prepare_graph", &c::graph::PREPARE_GRAPH),
        Entry::AcquireGraph => acquires(graph, "acquire_graph", &c::graph::ACQUIRE_GRAPH),
        Entry::AcquireGraphDomain => acquires(graph, "acquire_graph_domain", &c::graph::ACQUIRE_GRAPH_DOMAIN),
        Entry::GraphRow => acquires(graph, "graph_row", &c::graph::GRAPH_ROW),
        Entry::DemandAnswers => acquires(roots, "new", &c::graph::DEMAND_ANSWERS),
        Entry::Reach => acquires(roots, "reach", &c::graph::REACH),
        Entry::Axis => acquires(roots, "axis", &c::graph::AXIS),
        Entry::Nearest => acquires(roots, "nearest", &c::graph::NEAREST),
        Entry::PrepareProgramFile => acquires(setup, "prepare_program_file", &c::admission::PREPARE_PROGRAM_FILE),
        _ => unreachable!(),
    }
}
