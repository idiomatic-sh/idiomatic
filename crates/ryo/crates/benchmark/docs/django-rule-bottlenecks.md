# Django rule bottlenecks

Snapshot **2026-11-04T01:53:45.226121+00:00**: all **411 runnable rules attempted** over the same pinned **2,875 Django files**; **410 completed**, **ISC004 is censored beyond 360s**, and **I001/RUF100 remain two missing implementations** in the 413-slot inventory. Authorized progressive retries continue independently. A timeout is incomplete failure and a lower bound, never its actual cost. Results are diagnostic and **not reference-certified speed acceptance**.

The main cohort uses sealed source `db1f035022c4b758362a7f010226d7a853088b6d`, worker `68156078f5f7947d1ea178f39ceb67fa509698ae4d2465323cde15c0f92398f5`, frozen409 queries and one worker. E902/RUF200 use a **separate current411+namespace source cohort** (`7a097013`, worker `d025a7b0a13443fb36b9d70647744ddc6ccbd2526a8b71b9e542f42086656166`); their times must not be pooled as same-source comparisons. Both cohorts retain the pinned file manifest and explicit query/helper identities.

In db1, the application DAG is used for both historical flag settings. Survey flags `DAG=1/graph=0` enable optional completed node-result retention; `0/0` disables that retention and changes a stored digest marker. They do **not** select different executors. The same-worker full-corpus F841 retention control was essentially flat:20.988s off versus21.043s on, with exact complete report bags/coverage/files. A new unmerged typed-control source passes actual malformed/default/on/off/diagnostic/syntax controls; shared receipt-output environment cleanup remains open.

## Provisional measured ranking

Main-source completed timings are ranked below; ISC004 is shown separately as a censored outlier. CPU and peak for that row are observations before termination, not completed-workload costs. Setup versus execution time is not separately exposed by this sealed worker.

| Rule | Status | Wall s | CPU s | Peak MiB | Hot operation | Work amplification | Evidence | Shared fix |
|---|---|---:|---:|---:|---|---|---|---|
| ISC004 | Censored >360s; retry live | >360 | 359.893 observed | 133.6 observed | Indexed predecessor selfjoin | 30.67M matching pairs after 620 probes on 99,202-row inputs | [counter receipt](../results/django-bottlenecks-2026-11-04/progressive-survey/ISC004-operation-summary.json) | Generic ordered-existence/anti-selfjoin strategy; review pending |
| F841 | Complete, uncertified | 21.043 | 21.036 | 127.8 | Repeated kind Select scans | Multiple129,899-row passes; DISTINCT composition barrier | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Preserve dedup stage; unify owner admission/lifetime |
| F811 | Complete, uncertified | 19.470 | 19.467 | 137.1 | Binding helper/source joins | Source overlap with F841; not CPU attribution | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Measured binding-source cohort below |
| PT026 | Complete, uncertified | 18.361 | 18.311 | 1523.2 | Token-prefix/range source pattern | Source hypothesis; CPU not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Compare generic ordered-existence eligibility |
| PT025 | Complete, uncertified | 17.217 | 17.148 | 1566.0 | Token-prefix/range source pattern | Source hypothesis; CPU not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Compare generic ordered-existence eligibility |
| RUF028 | Complete, uncertified | 16.684 | 16.677 | 254.8 | Indexed joins/intermediate flow | 151,833-row intermediate;154,374 chain rows at one node | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Shared demand/range planning; not a rule workaround |
| PT020 | Complete, uncertified | 16.164 | 16.125 | 1575.6 | Token-prefix/range source pattern | Source hypothesis; CPU not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Compare generic ordered-existence eligibility |
| PLC0206 | Complete, uncertified | 14.752 | 14.749 | 141.6 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| PIE790 | Complete, uncertified | 13.971 | 13.969 | 103.5 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| PLW0406 | Complete, uncertified | 12.705 | 12.691 | 1976.6 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| UP008 | Complete, uncertified | 11.390 | 11.388 | 106.8 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| UP037 | Complete, uncertified | 11.079 | 11.079 | 116.4 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| PYI052 | Complete, uncertified | 11.012 | 11.004 | 127.9 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| RET501 | Complete, uncertified | 10.728 | 10.725 | 156.9 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| RUF019 | Complete, uncertified | 10.390 | 10.387 | 118.7 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |
| FA100 | Complete, uncertified | 10.381 | 10.378 | 109.1 | Not attributed | Not attributed | [resource receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/db1-resource-receipts.json) | Await evidence |

Separate-source E902 completed0.760s wall/0.756CPU/53.9MiB; RUF200 completed 0.773s wall/0.769CPU/54.1MiB. Each selected/evaluated all2,875 files with zero unknowns. See [separate cohort receipts](../results/django-bottlenecks-2026-11-04/progressive-survey/current411-resource-receipts.json).

## Family versus collective work

| Same-source measured cohort | Wall s | CPU s | Peak MiB | Report comparison |
|---|---:|---:|---:|---|
| F401/F811/F821/F841 binding-source4 |31.251|31.229|147.4|Exact isolated per-rule/skip bags; native syntax once|
| RUF028/RET501/SIM114/B018 mixed-flow4 |33.955|33.951|285.5|Exact isolated per-rule/skip bags; native syntax once|
| Original mixed12 |62.522|62.496|329.8|Exact isolated per-rule/skip bags; native syntax once|

These are direct cohort measurements, not sums/subtractions of isolated timings. They do not establish pathological collective overhead. ISC004 is a clear single-rule outlier; whether its ordered-existence family dominates the full workload still needs measurement. The shared-helper groups overlap in physical primitives; their labels do not imply disjoint costs.

## Confirmed mechanisms and proposed next step

ISC004’s counter capture is an **incomplete, instrumented prefix**, not a timing result:1,028 completed file scopes, zero overflow. In `tests/admin_views/tests.py`, rule-step35 selfjoins a99,202-row predecessor population by `(id,file)`. Its equality index already works, but broad same-key chains yield30,666,917 matching candidate pairs after only620 probes. The query then tests strict token-end ordering and antijoins away dominated predecessors. A generic ordered-existence/extrema physical strategy could avoid enumerating quadratic pairs while preserving maxima ties, bags and absence. Pure algebraic recognition belongs in domain planning; application owns strategy policy, acquisition, sharing and lifetime. No rule-specific rewrite, new cap, or new major implementation has been accepted.

F841 exposes a separate DISTINCT projection composition barrier. Mixed-rule selector histograms also show real budget refusal. At the first refusal the16MiB view pool is almost full; a3,122,479-byte equality index cannot fit. Existing ordinary-policy reclaim can release only657,312 physical bytes/32,768 position units; required/external aliases pin larger views. Graph/domain/nearest leases also occupy this ledger outside ordinary `State.entries`, so the ordinary top16 does not attribute the whole pool. Only graph acquisition currently invokes reclamation; selector prechecks and ordinary acquisition have separate admission decisions. These remain **unchecked ownership/admission/lifetime gaps**, not proof of stale accounting or a reason to raise caps.

All semantic capabilities remain in strong crates. Shared representations and service control flow should address copies/materializations; these captures measure row work, not allocation/retained-byte attribution, conversion copies, or index rebuild cost. Mandatory owners/interfaces remain required even when optional retention is disabled.

## Review and replay

The [evidence-backed hypothesis/plan](../results/django-bottlenecks-2026-11-04/progressive-survey/hypothesis-plan.md) includes ties, projected dedup keys, bag order, missing/malformed operands, lazy errors, injected refusal, cancellation and lifetime risks. Requested Fable high-effort review **did not run**: [actual response](../results/django-bottlenecks-2026-11-04/progressive-survey/fable-response.json) reports “weekly limit · resets Nov5,8am(UTC)”, zero review tokens. There is no Fable approval; major performance implementation waits for the requested review choice.

The [timestamped all413 inventory](../results/django-bottlenecks-2026-11-04/progressive-survey/inventory-state.json), [manifest](../results/django-bottlenecks-2026-11-04/progressive-survey/manifest.json), preserved survey/cohort drivers and separate source-cohort resource receipts bind current findings. Raw collector observations remain preserved under the local paths recorded in those receipts; this compact public snapshot is not a self-contained reference-parity audit. Replay requires the pinned corpus, frozen query/helper pack and sealed workers identified by their manifests. Each invocation takes workspace then serial locks; completed receipts are reused, and only timed-out rules retry30→90→180→doubling without a hard final cutoff.
