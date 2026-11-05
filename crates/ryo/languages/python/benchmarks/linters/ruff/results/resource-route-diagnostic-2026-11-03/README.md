# Resource route diagnostic — 2026-11-03

The [source patch](source.patch.gz) adds an opt-in, per-file route counter to the shared benchmark diagnostic and the application acquisition paths. Its disabled mode emits no route event. Core 499, benchmark 111, and capability 2+3+4 tests passed; [Fable review](fable-review.json) found no blocking source issue. The first compile refusal and [narrow correction](gate1-fix.patch.gz) are retained in the [raw receipts](raw.tar.gz). The [summary](summary.json) binds source and worker hashes.

The sealed worker `b11e2d12` ran frozen Django 3.10 `django/contrib/admin/options.py` with 310 selected rules plus the internal invalid-syntax rule, one thread, fresh caches, and DAG execution. Disabled and enabled runs exited 0 with identical output SHA-256 `c5de4440…`. Disabled emitted zero resource-route events; enabled emitted exactly one with 20,030 keyed rows, 116,133 recorded calls, and **zero overflow**.

| Route | Recorded calls | Successful physical builds |
| --- | ---: | ---: |
| Graph reach requests | 41,975 | — |
| Graph reach memo hits | 23,063 | — |
| Graph reach traversals | 18,912 | — |
| Representation inspections | 10,858 | — |
| Nearest requests / traversals | 5,645 / 5,643 | — |
| Owner acquisition | 1,426 | — |
| Owner build | 943 | 943 |
| Selector predicate / distinct builds | 685 / 354 | 685 / 354 |
| All other named physical builds | 106 | 106 |

Successful physical builds total **2,088**. The `unclassified` consumer label covers 112,402 of 116,133 calls; owner and plan IDs are known for only 2,895 calls. Unknown identities remain unknown in the evidence. These are one-file Python diagnostic counts, not CPU attribution, whole-corpus volume, or timing acceptance. A Rust-language input has not been run through this diagnostic.

Raw archive SHA-256: `10b945d66500747408223aca5ca0c3b22c164e49130a1fd8649126477a305f49`.
