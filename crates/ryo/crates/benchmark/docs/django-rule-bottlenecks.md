# Django rule bottlenecks

Provisional snapshot: 2026-11-04T01:11:52.420809+00:00. Diagnostic results, not reference-certified speed acceptance. All measurements use the same pinned 2,875 Django files, one worker, sealed source `db1f035022c4b758362a7f010226d7a853088b6d` and binary `68156078f5f7947d1ea178f39ceb67fa509698ae4d2465323cde15c0f92398f5`. [FileAnalysis in that source](../../core/src/application/file_analysis/service.rs) uses the same file DAG pipeline for both survey and control. Survey flags `DAG=1/graph=0` enable optional node-result retention; control flags `0/0` disable that retention. The flags also change a stored digest marker. A typed request choice must replace this environment selection and bind execution, store identity and receipts.

The inventory has 413 slots: 411 currently implemented publicly; this sealed cohort supports 409. E902/RUF200 await a compatible worker; I001/RUF100 are missing implementations. Reporting materializes every slot, including pending work.

Snapshot counts: COMPLETED_UNCERTIFIED: 226, PENDING_COMPATIBLE_WORKER: 2, FAIL_MISSING_IMPLEMENTATION: 2, FAIL_TIMEOUT: 1, PENDING_MEASUREMENT: 182. ISC004 exceeded the first 30-second stage: its runtime is censored, not measured as 30 seconds. Only timed-out rules progress to 90, 180 and subsequent doubled stages; completed rules are reused.

The table is the initial twelve-rule sample, not a ranking of all 226 completed rules. The full timestamped inventory is in the receipt bundle.

| Rule | Status | Wall s | CPU s | Peak MiB | Hot operation | Work amplification | Evidence | Shared fix |
|---|---|---:|---:|---:|---|---|---|---|
| F841 | Complete, uncertified | 21.043 | 21.036 | 127.8 | Repeated kind Select scans | Six or more distinct nodes scan the same 129,899-row source in admin_views/tests.py | [`F841` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Audit common selector/index eligibility; retention-disabled control found no meaningful single-pair difference |
| RUF028 | Complete, uncertified | 16.684 | 16.677 | 254.8 | Indexed joins and intermediate row flow | 151,833-row intermediate; 154,374 index-chain rows at one node | [`RUF028` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Inspect demand/seed scheduling and shared representations before proposing changes |
| RET501 | Complete, uncertified | 10.728 | 10.725 | 156.9 | Not attributed | Not attributed | [`RET501` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| F401 | Complete, uncertified | 9.830 | 9.817 | 103.6 | Not attributed | Not attributed | [`F401` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| B018 | Complete, uncertified | 8.571 | 8.565 | 179.3 | Not attributed | Not attributed | [`B018` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| PLR1708 | Complete, uncertified | 7.779 | 7.778 | 104.8 | Not attributed | Not attributed | [`PLR1708` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| F821 | Complete, uncertified | 7.066 | 7.066 | 105.8 | Not attributed | Not attributed | [`F821` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| SIM114 | Complete, uncertified | 6.393 | 6.390 | 139.9 | Not attributed | Not attributed | [`SIM114` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| UP034 | Complete, uncertified | 4.595 | 4.586 | 102.1 | Not attributed | Not attributed | [`UP034` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| UP031 | Complete, uncertified | 4.372 | 4.372 | 95.8 | Not attributed | Not attributed | [`UP031` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| B021 | Complete, uncertified | 3.496 | 3.496 | 90.6 | Not attributed | Not attributed | [`B021` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |
| UP006 | Complete, uncertified | 2.936 | 2.936 | 74.9 | Not attributed | Not attributed | [`UP006` receipt](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) | Await measured attribution |

Combined same-source twelve-rule cohort completed in **62.522s wall /62.496s CPU /329.8 MiB peak**. Per-rule diagnostic and skip bags match isolated reports; native syntax diagnostics occur once. This does not establish pathological collective overhead. Isolated timings must not be summed or subtracted to predict combined cost.

F841 and RUF028 operation captures completed all 2,875 files with zero overflow and exact default-worker report bags/coverage. Their instrumented elapsed times are not benchmark timings. Complete sealed JSONL was drained offline after worker exit. Counters establish rows examined/emitted, not CPU attribution, allocation bytes, retained backing, conversion copies or index rebuild counts; those remain unmeasured. Strong capability crates remain the intended boundary: address materialization costs through shared representations and service control flow.

Replay: run the source-bound survey driver from the linked archive with its retained manifest and frozen profiles; completed receipts are validated and reused. Per-invocation workspace then serial locks prevent competing timed jobs. The archive also includes the all-slot reporting companion. It preserves manifests and drivers; replay additionally requires the pinned corpus, frozen helper pack and sealed worker identified by those manifests.

The checked-in [receipt bundle](../results/django-bottlenecks-2026-11-04/receipts.tar.gz) contains the timestamped all-slot inventory, manifests, source-bound sample/cohort summaries, full drained operation summaries and preserved drivers. Retention-disabled F841 control completed in 20.988 s versus 21.043 s with retention enabled, with exact complete report bags, coverage and file list; this single pair provides no meaningful retention benefit.
