# Owner checkpoint: frozen workload measurement

- Sources: accepted [69a402d8](https://github.com/idiomatic-sh/idiomatic/commit/69a402d8df9d8864b51094c256a165883816aeb2) and corrected owner [f09b48b3](https://github.com/idiomatic-sh/idiomatic/commit/f09b48b3a924e20b36987f74fd6100dbbc84cd66). Release benchmark workers were sealed separately; same Django 3.10 corpus (2,875 files), 310 rules, one thread, and cache strategy.
- Reports match byte for byte in each pair: cold/warm 2,327 findings, edit 2,328; 891,250 selected, 890,943 evaluated, 307 skipped, zero unknown. [Six phase receipts](django/) retain source/worker hashes and loads. Each timing is one observation.

| Django phase | Base wall | Candidate wall | Candidate change |
| --- | ---: | ---: | ---: |
| Cold | 214.090 s | 240.497 s | +12.33% |
| Warm | 2.125 s | 2.068 s | −2.71% |
| Edit, retained cache | 8.945 s | 9.138 s | +2.16% |

- The shipped Rust `lint` bench target used all 244 unchanged recordings: cold median 1,587→1,658 ms (+4.47%), cold CPU 11,271→11,887 ms (+5.47%), spike CPU 2,160→2,520 ms (+16.67%). Warm 88→88 ms; edit 339→327 ms; peak 502→499 MB. [Rust summary](rust-summary.json) and [build identity](rust-manifest.json) preserve the single matched pair. Spike CPU exceeds the existing 1.15 ratio guard; the separate 100 MB warm peak gate remains open.
- Existing `profile-ryo` records only INFO spans: its [trace](chrome-trace.json) has compile/list/read/cross, with `read` as an umbrella. It cannot attribute CPU or allocations inside that span.
- A one-file [telemetry preflight](routes-smoke-summary.json) verified execution phases and resource routes. The full candidate-only [route summary](routes-summary.json) covers all 2,875 files with zero counter overflow. Instrumented phase sums: provider parse 8.529 s, native walk 0.409 s, rules/frontiers 293.764 s, settle/release 2.035 s; 54.181 s remains unattributed against 358.919 s diagnostic wall. Successful physical build events: PredicateMatches 1.172 million, Equality 759 thousand, DistinctRepresentatives 633 thousand. Counts and instrumented wall are not exclusive CPU cost or a baseline comparison.
- Raw JSONL remains local at `/tmp/owner-django-candidate-routes-20261103/events.jsonl` (17,913,391,401 bytes, SHA-256 `b03be17661963162ead2e68b250bbce5974e63de5d9e59799cc00148174c7137`); [receipt](routes-receipt.json) binds worker, profile, command and raw hash. The exact command is `diagnostic-ryo --request-file <frozen-request> python-target=3.10 <events.jsonl>` with `RYO_DIAGNOSTIC_EXECUTION=1`, `RYO_DIAGNOSTIC_RESOURCE_ROUTES=1`, `RYO_DIAGNOSTIC_STEPS=0`, and `RAYON_NUM_THREADS=1`.
- Performance acceptance remains open. Graph/fanout ownership is outside this checkpoint. Matched baseline route timings or a narrow shared phase timer are needed to explain the observed regressions.
