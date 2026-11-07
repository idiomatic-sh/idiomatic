//! The rows of the entries that acquire a view: each names its positive and negative compile control.
use super::super::*;

pub(super) fn row(entry: Entry) -> Row {
    let admission = owner!("acquisition/admission/service/issuance/service.rs");
    let acquisition = owner!("acquisition/service.rs");
    let raw = owner!("raw_scope.rs");
    let graph = owner!("graph.rs");
    let roots = owner!("graphs/types/roots/value.rs");
    let selection = owner!("selection/mod.rs");
    let setup = owner!("setup.rs");
    let owner = owner!("mod.rs");
    let cached: &'static [&'static str] = &[];
    match entry {
        Entry::AdmitView => acquires(
            admission,
            "admit_view",
            &c::admission::ADMIT_VIEW,
            &[
                file!("closure_partitions/schemas.rs"),
                file!("relations/selected_access.rs"),
                file!("workspace/mod.rs"),
                file!("workspace/service.rs"),
            ],
            ".admit_view(",
            "",
        ),
        Entry::AdmitPartition => acquires(
            admission,
            "admit_partition",
            &c::admission::ADMIT_PARTITION,
            &[
                file!("closure_partitions.rs"),
                file!("closure_partitions/schemas.rs"),
            ],
            ".admit_partition(",
            "",
        ),
        Entry::AcquireStored => acquires(
            acquisition,
            "acquire_stored",
            &c::admission::ACQUIRE_STORED,
            &[
                file!("closure_partitions/schemas.rs"),
                file!("workspace/service.rs"),
            ],
            ".acquire_stored(",
            "",
        ),
        Entry::AcquireStoredWithRoute => acquires(
            acquisition,
            "acquire_stored_with_route",
            &c::admission::ACQUIRE_STORED_WITH_ROUTE,
            &[file!("relations/selected_access.rs")],
            ".acquire_stored_with_route(",
            "",
        ),
        Entry::TryAcquireRows => acquires(
            acquisition,
            "try_acquire_rows",
            &c::admission::TRY_ACQUIRE_ROWS,
            &[file!("workspace/mod.rs")],
            ".try_acquire_rows(",
            "",
        ),
        Entry::ScopedBuild => acquires(
            owner!("acquisition/scoped/service.rs"),
            "scoped_build",
            &c::admission::SCOPED_BUILD,
            &[file!("closure_partitions.rs")],
            ".scoped_build(",
            "",
        ),
        Entry::FiniteRaw => acquires(
            raw,
            "finite_raw",
            &c::admission::FINITE_RAW,
            &[file!("program/kernel_session/positions/service.rs")],
            ".finite_raw(",
            "",
        ),
        Entry::WithRawOrder => acquires(
            raw,
            "with_raw_order",
            &c::admission::WITH_RAW_ORDER,
            &[],
            ".with_raw_order(",
            "",
        ),
        Entry::EvaluateNative => acquires(
            raw,
            "evaluate_native",
            &c::admission::EVALUATE_NATIVE,
            &[file!("program/kernel_session/positions/service.rs")],
            ".evaluate_native(",
            "",
        ),
        Entry::PrepareGraph => acquires(
            graph,
            "prepare_graph",
            &c::graph::PREPARE_GRAPH,
            &[
                file!("invocation/service.rs"),
            ],
            ".prepare_graph(",
            "",
        ),
        Entry::AcquireGraph => acquires(
            graph,
            "acquire_graph",
            &c::graph::ACQUIRE_GRAPH,
            cached,
            ".acquire_graph(",
            "",
        ),
        Entry::AcquireGraphDomain => acquires(
            graph,
            "acquire_graph_domain",
            &c::graph::ACQUIRE_GRAPH_DOMAIN,
            cached,
            ".acquire_graph_domain(",
            "",
        ),
        Entry::GraphRow => acquires(
            graph,
            "graph_row",
            &c::graph::GRAPH_ROW,
            &[file!("execution/lease.rs")],
            ".graph_row(",
            "",
        ),
        Entry::DemandAnswers => acquires(
            roots,
            "new",
            &c::graph::DEMAND_ANSWERS,
            cached,
            "DemandAnswers::new(",
            "",
        ),
        Entry::Reach => acquires(
            roots,
            "reach",
            &c::graph::REACH,
            cached,
            ".reach(",
            "DemandAnswers",
        ),
        Entry::Axis => acquires(roots, "axis", &c::graph::AXIS, cached, ".axis(", "DemandAnswers"),
        Entry::Nearest => acquires(
            roots,
            "nearest",
            &c::graph::NEAREST,
            cached,
            ".nearest(",
            "DemandAnswers",
        ),
        Entry::SelectorPlan => acquires(
            selection,
            "selector_plan",
            &c::selection::SELECTOR_PLAN,
            &[file!("program/selection.rs")],
            ".selector_plan(",
            "",
        ),
        Entry::Select => acquires(
            selection,
            "select",
            &c::selection::SELECT,
            &[file!("program/selection.rs")],
            ".select(",
            "derived",
        ),
        Entry::PrepareProgramFile => acquires(
            setup,
            "prepare_program_file",
            &c::selection::PREPARE_PROGRAM_FILE,
            &[file!("program/facade_runtime/file_execution.rs")],
            ".prepare_program_file(",
            "",
        ),
        Entry::PrepareViewsWithSchemas => acquires(
            owner!("selection/preparation/service.rs"),
            "prepare_views_with_schemas",
            &c::selection::PREPARE_VIEWS,
            &[file!("service.rs")],
            "prepare_views_with_schemas(",
            "",
        ),
        _ => unreachable!(),
    }
}
