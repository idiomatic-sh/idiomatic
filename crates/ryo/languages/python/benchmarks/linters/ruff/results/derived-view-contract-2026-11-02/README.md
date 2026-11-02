# Derived-view contract checkpoint — 2026-11-02

The application now prepares and owns the 12 derived-view families used by the file executor. The same source snapshot passed **485 core**, **2 derived-view capability**, **3 operator capability**, **4 representation capability**, **33 SDK**, **206 Python**, and **110 benchmark** library tests. The [summary](summary.json) binds the source snapshot, sealed release worker, commands, and [raw receipts](raw.tar.gz). These local gates establish behavior and capability boundaries; remote CI has not run on this source.

## Frozen Django comparison

The sealed worker `a6fb0df9838f0caca648eee664304dfe7630e8eebde1b69a929ed5d43b88d5f2` and retained relation worker `4c3825271775cabd1f72a71ac8d4af6353c3619ec4e9aff0f07d46ab72f9e9e1` used the same frozen **310-rule, 2,875-file** Django profile, Python 3.10 target, one Rayon thread, DAG setting, timing collector, and phase cache policy. Every new phase completed without a cap and produced a byte-identical normalized report to its retained counterpart.

| Phase | Relation wall | Derived-view wall | Wall change | Relation peak RSS | Derived-view peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Cold | 222.591 s | 242.139 s | 8.78% slower | 2,228,465,664 B | 2,302,504,960 B |
| Warm | 2.044 s | 2.057 s | 0.61% slower | 380,362,752 B | 410,906,624 B |
| Retained edit | 13.168 s | 12.767 s | 3.04% faster | 2,098,937,856 B | 2,129,686,528 B |

Cold and warm reports contain 2,327 findings; retained edit contains 2,328. Each has 890,943 evaluated rule-file pairs, 307 skipped, and zero unknown or unread. The new cold run is also slower than the older 214.557 s pre-contract observation. These are single serial observations with mixed direction and higher memory use. Performance acceptance remains open.

The earlier exact-head full-inventory CI result of **383 PASS / 30 FAIL** belongs to the [prior source](../ci-24274-full-inventory-2026-11-02/README.md). No 413-rule replay or remote Rust guard has yet run on this derived-view source.

Artifact identities: `summary.json` SHA-256 `d6863f467972fb45cdd4303bb36cc7d86bd6a4e9a3d4bb393792c633701db13a`; `raw.tar.gz` SHA-256 `0ab3da5e8491e1b5ef5c94d7b3c5bf2cf2485e2ab237f9ace88986c039973587`.
