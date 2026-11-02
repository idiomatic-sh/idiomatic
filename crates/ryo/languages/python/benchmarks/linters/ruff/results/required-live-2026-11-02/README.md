# Required-live operator checkpoint — 2026-11-02

The mandatory operator contract and required-live DAG retention pass 444 core, 3 capability, and 205 Python library tests. The sealed worker is `92b623314c19b12e0678f77b2248287a329e41254a1340b9ec1245b04641390f`. Timing used collector `4e1d3d328618bee455f9ee1f6e9c1963f076803e7251bdc953cb55db362d4205`; rule replay used shared inventory collector `813f50ad3cf909b23a48da6801a50d756c349ab47129e7dd07815c6e277dd004`. [Raw receipts and source identities](raw.tar.gz) and [machine-readable summary](summary.json) accompany this report.

## Frozen Django310 comparison

The same 310 rules, 2,875 files, parser target, one Rayon thread, DAG setting, timing collector, and fresh-cache policy were used. The retained pre-contract baseline is a separate earlier observation. All three new phases completed with full coverage and zero unknown or unread rule-file pairs. Structured reports match the baseline or frozen edited golden exactly; the receipt files use a different JSON object-key order.

| Phase | Baseline | Required-live worker | Result |
| --- | ---: | ---: | --- |
| Cold wall | 214.557 s | 244.698 s | **14.05% slower** |
| Cold CPU | 214.475 s | 244.552 s | **14.02% higher** |
| Cold peak RSS | 2,199,207,936 B | 2,270,191,616 B | **3.23% higher** |
| Warm wall | — | 1.979 s | Exact structured cold report |
| Retained edit wall | — | 13.722 s | Exact structured edited golden |

Cold and warm each report 2,327 findings; the retained edit reports 2,328. The earlier all24 worker had hit a 600-second cap on this profile. Required-live retention removes that failure, but the completed cold result still fails the performance comparison. These are single observations on separate runs; no stability claim follows from them.

## Scoped mechanism and rule proof

On `django/contrib/admin/options.py`, the [old capped diagnostic](../all24-contract-snapshot-2026-11-02/options-diagnostic.json) ran DAG node 26506 **11,970** times; the required-live diagnostic ran it **3** times and produced the exact old report. An ordinary, separate cold one-file pair took 3.457 s on the old worker and 3.254 s on the new worker; peak RSS rose from 402,640,896 B to 485,634,048 B. This one-file result does not explain the full-corpus cold regression.

A 45-second full-profile diagnostic reached 565 files and emitted 4,856,658 DAG-node events. Within any sampled file, the maximum `(file, node)` repeat count was 16. JSON event writing produced 1.87 GB, so these event counts are work-shape evidence rather than ordinary timing attribution. The [bounded trace summaries](raw.tar.gz) retain the top nodes, files, sampled rule durations, cap boundary, and exact hash of the local raw trace. The 1.87 GB trace is omitted from Git.

RUF101 passed all 40 current-helper cases with raw candidate/reference parity on this worker. Its [source-bound supplement](../full-inventory-current/ruf101-target-supplement/README.md) updates the pinned [413-rule table](../../RULES.md) to **378 PASS / 35 FAIL**. N999 remains FAIL: 44 of 47 candidate cases passed; the unresolved cases are a nested dotted initializer and `.txt` and extensionless inputs rejected as `NoLanguage`. The lexical recovery change resolved its prior invalid-syntax case.

The next engine work must account for broad per-node execution and representation cost while retaining the proven result and lifecycle behavior. The current full-corpus performance result is not accepted.
