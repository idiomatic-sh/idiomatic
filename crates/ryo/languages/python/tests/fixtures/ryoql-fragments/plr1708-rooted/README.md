PLR1708 sparse filtered traversal
===============================

The rule preserves its exact computed edge exclusions at nested functions,
lambdas and embedded-expression hosts. It walks only real function-body roots,
through every intermediate node, filtering emission to yield/raise members.
Original direct body members and downstream file keys remain intact.

All language NodeIds include their FileId. This permits these per-file semantic
edges to use the existing Axis operator; it does not make Axis a partitioned
closure for arbitrary hosted graphs. The generic probe compares counted
(root,file,payload,node) membership against the old file-keyed closure on real
nodes, two files with equal local positions, and unrelated width/depth growth.
The rule's downstream tests are membership-only. BFS versus sorted closure
intermediate ordering is not asserted equivalent.

proof.json binds 28 cases/56 complete reports on sealed worker 2ea: all 17 old
cases retained plus 11 boundary controls. Existing sources reuse their pinned
expectations; new-oracle-records.json retains ten fresh Ruff0.16.5 runs and one
disjoint two-file composition. No whole-corpus or wall-time claim is made.

The Ruff bench's engine test
`plr1708_rooted_equivalence_reports_nothing_across_unrelated_growth` runs
equivalence.ryoql over its eight corpora on every test run, and
`actual_rooted_queries_keep_their_sparse_axis` holds the rule query to its
rooted walk (see sparse-traversal-guards/README.md). The core Axis case added in
this batch runs in the existing `succeeds` unit test and checks the visited set
remains independent of unrelated components, including cycles and file IDs.
