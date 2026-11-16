Sparse traversal guards
=======================

The PIE794, B023/C419 and PLR1708 rooted queries answer by membership and counted
multiset, so a rooted axis must report exactly what the global closure over the
same roots reports, however much unrelated code grows around them. Four engine
tests of the Ruff bench (`languages/python/benchmarks/linters/ruff/engine_test/sparse_traversal_test.rs`)
hold that on every test run, through the real Python provider:

- `pie794_rooted_equivalence_reports_nothing_across_unrelated_growth`: the 8
  corpora of pie794-rooted (two contexts, two files sharing node identities,
  unrelated width 16/256/1024 and depth 4/16/48);
- `sparse_descendant_consumers_equivalence_reports_nothing_across_unrelated_growth`:
  the same 8 plus an await inside and an await outside the roots (10);
- `plr1708_rooted_equivalence_reports_nothing_across_unrelated_growth`: the 8
  corpora of plr1708-rooted, its scope boundaries filtered;
- `actual_rooted_queries_keep_their_sparse_axis`: the rule queries keep the
  rooted `axis(...)` the equivalences prove and never the global closure.

Each corpus must report no diagnostic and account for every rule/file pair with
none unknown or unread. Each fixture directory keeps its equivalence.ryoql and
baseline files; the tests read them with `include_str!`. They assert membership
and bags, not equivalence of BFS and sorted-closure physical order. The separate
core Axis `succeeds` test includes a rooted visited-set work guard independent of
unrelated graph components.

executions.json records the earlier Python runner's check on worker2ea; that
runner (`run.py`) and its three `generic.py` probes were replaced by these tests,
and commit `981eb0d869` keeps them.
