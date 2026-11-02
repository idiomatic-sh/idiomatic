# Complete pinned Ruff inventory, 2026-11-02

**364 PASS / 49 FAIL / 413 defaults in current source-bound evidence.** This combines the completed **362 PASS / 51 FAIL** shared benchmark inventory run with [two later 17/17 singleton replays](sim222-sim223-supplement/summary.json) on the same sealed worker. It is not a single 364-rule execution. The full collector exited 1 because 51 rules failed at that source snapshot. [Its summary](summary.json) has one PASS or FAIL row per pinned rule; [raw receipts](raw-receipts.tar.gz) retain the generated registration, native case JSONL, terminal row results, and child streams. [The supplement proof](sim222-sim223-supplement/proof.json) binds the unchanged base summary and separate [raw receipts](sim222-sim223-supplement/raw-receipts.tar.gz).

In the complete base run, the 51 failures comprised 30 missing implementations, 19 verifier failures, and 2 missing or ambiguous settings registrations (SIM222, SIM223). Passing base rows executed 8,837 case pairs: 8,753 raw exact pairs and 80 independently documented reference differences. The two supplemental rules add 34 raw-exact case pairs. Failed rows retain their error details in the base summary and archive.

| Identity | SHA-256 |
|---|---|
| Pinned Ruff 0.16.5 executable | `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8` |
| Sealed Python worker | `d43a5760d6c9388d7ba307416417268fbf1cacb92ae6fbe9bf70dffdf0d686b8` |
| Corrected shared collector | `813f50ad3cf909b23a48da6801a50d756c349ab47129e7dd07815c6e277dd004` |
| Summary JSON | `a3eae09e90dcb4a2d7a88bb7e76fddc9888b31648133838fcb1edee3b11e6f91` |
| Raw receipts archive | `3f36a818e9addd6be75462f011656ddb3193c4c0b0f5b749bee84d74383b0bdc` |

The runner initialized all 413 rows as FAIL, checked each registration in pinned order, recorded each result atomically, and reached `run_state=complete`. Its summary binds the inventory and phase registry hashes; each passing row binds its query, cases, settings, imported modules, and native receipt digest. The worker used the default execution strategy. This single run establishes fixture results for this source and environment; it does not establish performance or cross-platform parity.
