# Mandatory operator contract diagnostic snapshot

The 2026-11-02 source snapshot routes all 24 operator kinds through the common application contract. Its focused behavioral gates passed: 440 core library tests, two compiler capability tests, 106 shared benchmark tests, one ModuleIdentity test, and nine Python module-context tests. These results establish the tested behavior and boundary; they do not establish acceptable full-corpus performance.

On the frozen Django Python 3.10 workload (2,875 files, 310 rules, one Rayon thread, graph execution enabled), both sealed workers used the same profile, corpus, validator, timing collector, and independent fresh caches:

| Worker | Wall time | CPU | Report |
| --- | ---: | ---: | --- |
| Pre-migration baseline `10c69096…` | 214.557 s | 214.475 s | Complete; 2,327 findings, 307 syntax skips, zero unknowns; SHA `02b96f5f…` |
| Contract candidate `28a3e2cd…` | Capped at 600.041 s | 599.721 s before termination | Absent |

The candidate exceeded **2.796×** the baseline wall time before the cap. Its observed 509,669,376 B RSS is only a peak **up to termination**, so it cannot support a completed-run memory comparison. Full-corpus report equivalence remains unverified for this worker. The contract migration is a performance regression at this snapshot and is not accepted as a performance improvement.

A separate 45-second diagnostic invocation is **not a timing sample**. It emitted 61,097 rule/file starts and 61,096 ends across 197 files. The last unmatched start was `PLC0206` on `django/contrib/admin/options.py`; the first event appeared promptly. This localizes the short-run stall to a rule/file invocation, without identifying the responsible operator or cost.

A second bounded diagnostic used only that file with the same 310 rules. The old worker completed; the candidate hit a 60-second diagnostic cap. Candidate DAG node `26506` executed 11,970 times and emitted 91,053 cells each time (1,089,904,410 cells total) while retained graph bytes stayed near the 6 MiB limit. The old worker's most repeated DAG node executed 110 times. This supports a repeated shared-node computation under the retention budget; the exact node-to-operation mapping and causal repair remain open. [Filtered events and compiled PLC0206 plan records](options-diagnostic.tar.gz) are indexed by [the diagnostic summary](options-diagnostic.json).

The cross-version retained-cache control ran the same PT010 query and source with the old worker, then the new worker twice: selected-rule executions were **1, 1, 0**. All three reports were byte-identical. This is actual old-schema invalidation followed by new-schema warm reuse.

N999 remains a candidate: its first new-worker replay checked 47 cases and failed four (one partial syntax case, one dotted initializer mismatch, and two non-Python-suffix files reported `NoLanguage`). These cases remain in the suite.

[Machine summary](summary.json) · [Hash index](evidence-index.json) · [Raw requests, logs, reports, and bounded diagnostic events](raw.tar.gz).
