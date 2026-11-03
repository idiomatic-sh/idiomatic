# Demand admission checkpoint — 2026-11-03

The application now reuses immutable request selector descriptors, prepares repetition once, accounts retained row slots in constant time, and reuses one demand-scoped input proof across derived-view acquisitions. This checkpoint also adds optional Python bytes-literal and format-specification facts. The [summary](summary.json) binds the source snapshot, release worker, gates, frozen profile, and [raw receipts](raw.tar.gz).

The combined source passed **491 core**, **2 derived-view capability**, **3 operator capability**, **4 representation capability**, **67 CLI**, **110 benchmark**, **98 language**, **223 Python**, and **33 SDK** tests. Earlier compile and fixture failures and their corrected retries remain in the archive. The two new rules have completed targeted execution, but their full source-bound profile admission and a 413-rule replay on this source are still pending.

## Frozen Django comparison

The sealed worker `0c8585f02b5ea65162237296cd7ea48a2db11d84b00ae5479c205eb55e5f6874` ran the same **310-rule, 2,875-file** Django profile as the [derived-view checkpoint](../derived-view-contract-2026-11-02/README.md): Python 3.10, one Rayon thread, DAG execution, identical frozen corpus, timing collector, and phase cache policy. All three phases completed within the 600-second cap. Each normalized report is byte-identical to its frozen relation and derived-view counterparts, including 2,327 cold/warm findings and 2,328 retained-edit findings. Coverage remained 890,943 evaluated rule-file pairs, 307 skipped, and zero unknown or unread.

| Phase | Derived-view wall | New wall | Wall change | Derived-view peak RSS | New peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Cold | 242.139 s | 227.084 s | 6.22% faster | 2,302,504,960 B | 2,297,688,064 B |
| Warm | 2.057 s | 2.073 s | 0.80% slower | 410,906,624 B | 410,914,816 B |
| Retained edit | 12.767 s | 10.974 s | 14.04% faster | 2,129,686,528 B | 2,128,875,520 B |

The new cold run is still **2.02% slower** and uses **3.11% more peak RSS** than the earlier 222.591-second relation checkpoint. These are single serial observations of a combined Python and engine source change; they do not isolate the engine fix or establish performance acceptance. A bounded callchain sampling attempt ended before starting the worker because this environment denied `perf` events (`perf_event_paranoid=2`); its failure receipt is preserved.

The last published exact-head full inventory, **383 PASS / 30 FAIL**, belongs to [earlier source](../ci-24274-full-inventory-2026-11-02/README.md). No 413-rule CI result or remote Rust speed verdict for this checkpoint is asserted here.

Artifact identities: `summary.json` SHA-256 `6b505706c945340e2d50e227b1640ed6cdbb24bd9160d2b29f3972f5760f0b5b`; `raw.tar.gz` SHA-256 `5253e0efb5ea0b2809cbec97588145bbde8f462042cc97b8ab718c0ced3c8c9c`.
