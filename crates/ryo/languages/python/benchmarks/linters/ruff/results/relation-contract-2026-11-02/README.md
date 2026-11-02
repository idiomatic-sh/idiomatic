# Relation contract checkpoint — 2026-11-02

The engine now admits stored, selected, native tree, and computed rows through one required representation contract. The common application owner controls source generations, publication, and relation lifetime. Focused gates passed **463 core**, **7 capability**, **33 SDK**, **206 Python**, and **107 benchmark** library tests. These include the real production routes and compile controls that reject primitive access to scheduling, cache, and persistent source lifetimes. The [machine-readable summary](summary.json) and [raw receipts](raw.tar.gz) bind the tested source, worker, requests, reports, and test output.

## Frozen Django comparison

The sealed relation worker `4c3825271775cabd1f72a71ac8d4af6353c3619ec4e9aff0f07d46ab72f9e9e1` and retained required-live worker `92b623314c19b12e0678f77b2248287a329e41254a1340b9ec1245b04641390f` used the same frozen **310-rule, 2,875-file** profile, Python 3.10 target, one Rayon thread, DAG setting, timing collector, and phase cache policy. The source manifest SHA-256 is `4ce6972fc36ae4728fae24d46175a6fd6248ee112e7f1aa4e39e7d83f5967a49`. Each candidate phase exited successfully without a cap and produced a byte-identical normalized complete report to its retained phase.

| Phase | Required-live wall | Relation wall | Wall change | Required-live peak RSS | Relation peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Cold | 244.698 s | 222.591 s | 9.03% faster | 2,270,191,616 B | 2,228,465,664 B |
| Warm | 1.979 s | 2.044 s | 3.29% slower | 375,140,352 B | 380,362,752 B |
| Retained edit | 13.722 s | 13.168 s | 4.04% faster | 2,092,859,392 B | 2,098,937,856 B |

Cold and warm reports contain 2,327 findings; retained edit contains 2,328. Coverage is identical across workers, with 890,943 evaluated rule-file pairs, 307 skipped, and zero unknown or unread. The candidate cold run remains **3.74% slower** than the older pre-contract 214.557 s observation. Each phase here is one run, so the mixed warm/edit direction does not establish a stable speed or memory gain. Remote Rust performance and platform guards have not run on this candidate source.

## Affected rule check

The new explicit Python source-language setting lets N999 complete **46 of 47** candidate cases; all 47 reference cases pass. The remaining `nested-dotted-initializer` case emits one extra candidate finding, so N999 remains **FAIL**. This was a targeted singleton selection using the shared inventory runner, not a full 413-rule replay. The archived summary retains the other 412 rules as unselected failures; those rows are not a new correctness result.

Artifact identities: `summary.json` SHA-256 `ddec6028bbd16da850e185c05fddbe93fd13dcf58651375c5c18c21edc4c1a02`; `raw.tar.gz` SHA-256 `cbf1dd1cd85a6a14efbc6cf9342919468f67e87e1cfb6ac558364d89b5dfa1b7`.
