# Discarded FileProgram pass POC — 2026-11-03

The [four-path source patch](source.patch.gz) removes construction of a FileProgram whose result production discarded. The active completeness calculation and DAG stay in place. The [Fable review](fable-review.json) found no source blocker. Core (494), capability (2+3+4), benchmark (110), Python (225), SDK (33), and focused Ruff engine (13) tests passed. The [summary](summary.json) and [raw receipts](raw.tar.gz) bind the source, sealed workers, tests, and measurements.

The accepted worker `82341f24` and candidate `052f49af` ran the same frozen Django profile: 310 rules, 2,875 files, Python 3.10, one Rayon thread, DAG execution, and isolated caches. All phases completed within the 600-second cap with byte-identical reports and coverage (890,943 evaluated, 307 skipped, zero unknown). The options.py diagnostic had identical output and DAG counters; the removed constructor was discarded work.

| Phase | Accepted wall | Candidate wall | Candidate change | Accepted peak RSS | Candidate peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Cold, one matched pair | 226.734 s | 211.381 s | 6.77% faster | 2,341,928,960 B | 2,310,823,936 B |
| Warm, one matched pair | 2.192 s | 2.071 s | 5.49% faster (0.120 s) | 411,160,576 B | 411,127,808 B |
| Retained edit, pair 1 | 9.251 s | 9.877 s | 6.78% slower | 2,129,960,960 B | 2,129,924,096 B |
| Retained edit, pair 2 | 11.667 s | 9.096 s | 22.04% faster | 2,129,915,904 B | 2,129,678,336 B |
| Retained edit, pair 3 | 10.909 s | 15.137 s | 38.75% slower | 2,130,108,416 B | 2,129,793,024 B |

For each edit repeat, both corpora and retained caches were restored to verified pre-edit snapshots. Pair order alternated. The large edit variance prevents a reliable retained-edit speed conclusion; the cold gain is one observation. This POC does not complete the common ownership contract or the 20–50% whole-workload target.

Archive SHA-256: `12662a7e8d60d0c80019d3da35465c4afd84958dee87288697f186e11d22f9e7`.
