//! What the cycle search over ruff costs beside the walk it searches: the same steps with the search swapped for a pass of its edges.

use crate::base;
use std::sync::Arc;

/// How many times the wall time of the walk the module-cycles recording over ruff may take. `cycles` is one pass over the nodes and
/// edges (Tarjan's), so the recording costs about what the same steps cost with the search replaced by a plain pass of the edges:
/// measured on the alpha3 tree, 282 ms against 303 ms for the walk. A search that grows with the pairs it reaches, such as `closure`
/// filtered to equal columns, took 7398 ms against 308 ms for the walk in this test, twenty-four times as long. The bound is a ratio of two runs in the
/// same process over the same files, so it holds on a slow machine as on a fast one, with room for the noise of a busy one.
const CYCLES_OVER_WALK_BOUND: u128 = 3;

/// The search the recording runs.
const SEARCH: &str = r#"cycles(edges=edges, source="from", destination="to")"#;

/// The plain pass of the search's edges that replaces it for the walk.
const PASS: &str = "select * from edges";

/// Lints the snapshot of ruff with the rule `query` alone and gives the wall time in milliseconds and how many rows it gave.
fn wall_of(name: &str, query: &str) -> (u128, usize) {
    let started = std::time::Instant::now();
    let spill = tempfile::tempdir().expect("a directory");
    let (rows, _) = base::lint_with(
        &base::snapshot::ruff(),
        vec![crate::rule_from_query(name, query, &["**".to_owned()])],
        Arc::new(ryo_execution::outbound::facts::factory::spill(
            spill.path().to_path_buf(),
        )),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    (started.elapsed().as_millis(), rows.len())
}

#[test]
fn module_cycles_over_ruff_costs_about_what_the_walk_of_its_edges_costs() {
    let _alone = crate::alone();
    let recorded = base::text(&base::directory().join("module-cycles/query.ryoql"));
    assert!(
        recorded.contains(SEARCH),
        "the recording searches with `{SEARCH}`"
    );
    let walk_query = recorded.replacen(SEARCH, PASS, 1);
    // A first walk reads ruff into the page cache, so neither timed run pays for the disk.
    let _warm = wall_of("warm-up", &walk_query);
    let (walk_wall, _) = wall_of("walk", &walk_query);
    let walk = walk_wall.max(1);
    let (search, found) = wall_of("module-cycles", &recorded);
    assert!(found > 0, "the search finds the cycles of ruff");
    eprintln!("module-cycles over ruff: walk {walk} ms, search {search} ms");
    assert!(
        search <= walk * CYCLES_OVER_WALK_BOUND,
        "module-cycles over ruff takes {search} ms, more than {CYCLES_OVER_WALK_BOUND} times the {walk} ms of the walk of its edges"
    );
}
