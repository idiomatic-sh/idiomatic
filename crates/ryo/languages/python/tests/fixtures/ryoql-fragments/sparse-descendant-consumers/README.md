C419 roots descendant traversal in actual comprehension elements and emits only
await nodes. Direct element roots remain in the union, so a root await is retained.
The domain restricts emitted nodes, never traversal through intermediate nodes.
All later await/async-generator logic is unchanged and uses set membership.

B023 replaces ALL THREE uses of the global semantic-child closure: loop target,
body and iterator roots. Its domain is `select child as id from semantic-child`,
the exact positive endpoint universe, avoiding assumptions about binding-event
occurrences or semantic-syntax visibility. Root rows and loop/file payloads remain
in their unions. Later joins/antijoins classify membership; no intermediate order
is consumed. Breadth-first axis order is not claimed to equal closure entity order.
No generic closure behavior, PLR1708 boundary traversal or inheritance traversal
was changed. Both global baselines and every prior case remain in this fixture.

55 cases / 110 complete real baseline/candidate workers pass exact expected and
full-report comparisons, zero Unknown, canonical phase/target/context/flags.
Nineteen cases were appended to the original36; seventeen fresh pinned Ruff0.16.5
PY310 calls supply the new arrays, plus two disjoint two-file compositions. No
unchanged oracle was rerun. Evidence: `/tmp/sparse-consumers-rule-proof-20261101`.

Ten generic counted-multiset cases check name, all-positive-endpoint and await
domains against global closure, repeated root IDs with different payload tags,
two files, a nonempty await domain and unrelated awaits, widths16/256/1024 and
depths4/16/48. A fixed twelve-name-pair assertion prevents vacuous success. Scoped
source guards pin these sparse consumers; they do not prohibit legitimate closure.
Evidence: `/tmp/sparse-consumers-generic-20261101`.

Instrumented large-file checks use the same2ea worker and a small first-file warmup.
B023 admin-views rule interval811.705 ->58.940ms. C419 schema interval284.487 ->22.744ms:
the old closure materialized510,853 pairs for one root/21 consumed descendants;
closure256.692ms becomes a15.333µs axis. These are diagnostic intervals, not paired
wall claims. C419 admin-views is explicitly a no-gain control (18.626 ->20.043ms):
its await-domain branch was empty and no old global closure executed at runtime.
The separate schema/nonempty-domain cases rule out inferring a win from that control.
Artifacts: `/tmp/sparse-consumers-tail-20261101`, `/tmp/c419-schema-tail-20261101`.

Paired native timing is pending an explicit quiet window after the324-rule
correctness run; no measurement is claimed under its four-worker load. No full
600second corpus rerun or new engine build was performed for this query batch.
