PLC0206 last-event selection regression guards.

The only rule expression changed is `older_file_points`. Previously it joined every point with every point in the same file, then selected smaller left points. It now performs ordered self-matching and retains the same final antijoin.

For distinct sorted points p0 < p1 < ... < pn in one file, greedy strict-later matching pairs pi with p(i+1), for every i < n. Its left projection is therefore exactly the old set of nonmaximum points. The final antijoin uses the identical left relation and membership keys, preserving its ordered multiset. Empty and singleton groups work without a special branch. There is no assumption that event ordinals are dense or that a count equals the maximum: the provider assigns schedule-cursor ordinals, including control gaps and repeated ordinals from expanded global/nonlocal declarations.

The actual worker plan confirms that `file_event_points` projects BindingEvent columns5/6 with `distinct: Dropped` before both ordered-match inputs. Both columns are Shared in the catalog; no declaration identity is retained by this projection. The duplicate-points fixture independently asserts this compiled deduplication behavior.

Validation on the same actual worker2ea:

- All42 prior full-report Ruff0.16.5/PY310 cases and8 additional source controls pass:50cases/100complete side records, no Unknown. New controls include empty events, final global/nonlocal alias ties, schedule-control gaps, independent files, repeated scoped identifiers and suppression. Exact staged query/module/case hashes are in `case-proof.json`.
- `verify.py` compares old/new typed point/file rows and multiplicities using symmetric antijoins of grouped counts. Nonempty fixtures also assert one result per input file to prevent vacuous success. Its5 cases cover empty, singleton, gaps, multiple files and duplicate points; `generic-proof.json` records their reports and compiled dedup guard.
- The RyoQL literal grammar accepts digits, so negative numeric fixtures are not synthesized through string encodings. A signed, unsorted, partitioned self-match case is added to the existing core ordered-match fixture list. It awaits the next required integration Rust test gate; no standalone engine worker build was requested. The existing suite already includes the full signed i64 ordering domain.
- The complete310-rule `options.py` report remains exactly equal:310evaluations, zero Unknown/skips and the same2diagnostics. Only the PLC0206 query differs from the accepted72ea suppression profile. The former R=3117,L=3117,H=9,715,689 join has no corresponding event after the rewrite. Total PLC0206 generic join hits fall from9,814,031 to98,342. `full-counts-proof.json` seals all310query hashes, module hashes, request identity, reports and event hashes. This is operation-count evidence, not a wall-time benchmark.

Run `verify.py --worker <existing-actual-worker> --output <artifact-directory>` for the generic checks. The source/case profile and full workload requests used here remain under `/tmp/plc0206-ordered-max-20261101`. No full-corpus timing result is implied.
