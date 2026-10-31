# Ruff subset benchmark: first corpus evidence

This is an interim comparison of **15 of Ruff 0.16.5's 413 shipped-default rules**, not default-rule parity. Both contestants explicitly select the same 15 rules and consume identical source bytes. All 206 independently authored cases passed before corpus execution. The full run exited **1**: Django and FastAPI passed; pandas exposed three false positives in B002. The exact mismatches are in `correctness.json`; they remain a correctness failure, not accepted differences.

The captured profile had 25 assessed deferrals, 15 other authored rules outside this benchmark, and 358 still-unassessed defaults. Later inventory work does not retroactively change this snapshot. Selected rules: B002, B013, B015, B016, B021, B026, B029, B032, E722, F631, F634, F707, PLW0129, PLW0131, SIM208.

| Project | Tool-cache condition | Ruff wall ms | Ryo wall ms | Ryo / Ruff | Peak RSS MiB Ruff / Ryo |
| --- | --- | ---: | ---: | ---: | ---: |
| django | empty-tool-cache | 294.82 | 4802.56 | 16.29× | 31.6 / 205.3 |
| django | retained-tool-cache | 33.88 | 38.83 | 1.15× | 25.4 / 44.6 |
| django | edited-tool-cache | 32.88 | 44.95 | 1.37× | 25.4 / 47.2 |
| fastapi | empty-tool-cache | 61.87 | 874.37 | 14.13× | 22.5 / 97.2 |
| fastapi | retained-tool-cache | 14.53 | 20.59 | 1.42× | 16.3 / 14.5 |
| fastapi | edited-tool-cache | 15.60 | 42.43 | 2.72× | 19.5 / 35.1 |

Each row is the median of six observations alternating contestant order. `measurements.json` includes min/max bounds and CPU time. `observations.log` retains every sample, exact source counts/digests, exclusions and per-rule finding counts. No pandas timing was certified. Empty means a new tool cache, not a cold OS page cache. Retained-cache correctness and edited-cache correctness were compared with independent clean executions; cache hits were not instrumented.

The full Python-input attempt first failed on Django's intentional `tests/test_runner_apps/tagged/tests_syntax_error.py`: Ryo reports requested relations as partial instead of matching Ruff's syntax-error findings. The subsequent timing profile explicitly excludes files rejected by the pinned Ruff parser from **both** contestants, recording diagnostic ranges. Ruff-valid files are never excluded just because Ryo cannot parse them. Other Ruff-default-discovered formats (notebooks/TOML) are also inventoried separately. Frontend parity remains open under `ryo-7tkn.2`.

Execution uses Linux aarch64, release optimization level 3, one processing worker each, and the pinned GNU time collector. Wall time includes collector/child startup, query loading/compilation and serialization; normalization, artifact hashing and source hashing are outside timing. CPU is GNU time user+system CPU with hundredth-second precision: a reported zero is below measurement resolution, not zero work. Peak RSS is the OS maximum resident set, not cumulative allocations. `environment.txt` records the toolchain and build flags; the immutable profile and executable hashes appear in the observations.

The observed weakness is empty-tool-cache cost: Ryo takes about 14–16 times the wall time and substantially more peak memory on the two successful corpora. Retained-cache cost is much closer, about 1.15–1.42 times Ruff. Measured performance follow-on `ryo-7tkn.19` records the acceptance criteria. This evidence motivates profiling parse/model materialization, query planning/evaluation and cache serialization separately; it does not attribute the slowdown to any one phase. An independently identified design issue, unconditional lexical-index construction even for syntax-only rules, is tracked in `ryo-7tkn.6` and still needs profiling before a speedup is claimed.

Tastemaker architecture work and full-default assessment remain incomplete. This report is evidence for the draft PR, not readiness approval.
