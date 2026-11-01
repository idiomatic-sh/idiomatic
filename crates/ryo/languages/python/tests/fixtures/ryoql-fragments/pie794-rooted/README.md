PIE794 now traverses from its actual candidate expression roots with the existing
`axis` operator and emits only name-expression descendants. It retains the original
roots in `all_values`. All intermediate nodes remain traversable. The duplicate
class-field algorithm, explicit source-position comparison, suppression call and
emit key are unchanged. This is an independent query algorithm preserving observable
behavior; Ruff 0.16.5 is the oracle, not an implementation template.

The old whole-AST closure is retained in `baseline.ryoql`. On the completed large
Django admin-views file (381,804 bytes), it materialized 797,534 positive descendant
pairs for six expression roots; only 52 pairs were consumed. The instrumented
closure interval was 726.275 ms of a 756.446 ms rule interval. Rooted traversal was
3.625 ms, with the rule interval 30.457 ms. Those are diagnostic intervals, not
independent wall samples. The small initializer was processed first in the same
worker to distinguish cached layout startup from tail runtime. Suppression mapping
still did 148² comparisons, but its closure was only 1.217 ms; this concrete evidence
prioritized expression traversal over further mapping factoring.

All 17 existing cases remain byte-preserved in `baseline-cases.toml`; 13 controls
were appended. Twelve fresh pinned Ruff/PY310 oracle calls cover direct roots,
intermediate/default/body/lambda/comprehension/format-spec names, syntactic shadowing,
Unicode/multiline locations, annotations and separate owners. A two-file case is a
disjoint rename of two independently checked source arrays. No unchanged oracle was
rerun. All 30 cases / 60 real baseline/candidate processes pass complete-report,
canonical phase/target/context/flags and exact expected-diagnostic comparisons.
Raw evidence: `/tmp/pie794-rooted-rule-proof-20261101`.

Intermediate order is deliberately NOT equated: global closure sorts entities,
while axis traverses breadth-first. Downstream self-reference rows are deduplicated
IDs used only for antijoin membership. That antijoin retains the original candidate
left rows; duplicate detection then uses explicit source positions. Eight generic
counted-multiset probes retain repeated root IDs with distinct payload tags, two
files, and unrelated width16/256/1024 and depth4/16/48. Every file must produce
exactly twelve reachable-name/payload pairs, making empty or pruned traversals fail.
The scoped sparse-consumer source guard prevents this query from reintroducing the
global expression closure; it does not ban legitimate closure. Evidence:
`/tmp/pie794-rooted-generic-20261101`.

Quiet native-collector measurements use the same sealed 2ea worker and immutable
frozen142 profile, with ONLY these two PIE794 lines changed. Three alternating pairs
retain all twelve samples, exact reports, fresh absent CLI caches and uncontrolled
OS caches. Single-rule warmup+tail median wall: 0.932598 -> 0.195501 seconds (-79.04%);
RSS:151,257,088 ->71,163,904 bytes (-52.95%). The SEPARATE completed forms file with
all310 rules:8.807016 ->8.486644 seconds (-3.64%), RSS effectively unchanged; all16
diagnostics match. These bounded workloads do not establish whole-corpus completion,
413-rule parity or subsecond project latency. `measurement-source.py` is the exact
as-executed supervisor, not a portable production runner; all requests, native
observations and immutable variant are `/tmp/pie794-rooted-paired-20261101`.
