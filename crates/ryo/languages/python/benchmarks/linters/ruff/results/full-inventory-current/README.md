# Complete pinned Ruff inventory, 2026-11-02

**379 PASS / 34 FAIL / 413 defaults in current source-bound evidence.** This combines the immutable [362 PASS / 51 FAIL full run](summary.json) with separate [SIM222/SIM223](sim222-sim223-supplement/summary.json), [future directive](future-directive-supplement/summary.json), [match OR and return type](match-or-supplement/summary.json), [FA100/FA102](fa-target-supplement/summary.json), [UP017](up017-target-supplement/summary.json), [UP035/UP036](up035-up036-target-supplement/summary.json), [RUF101](ruf101-target-supplement/summary.json), and [PLE0116](ple0116-target-supplement/summary.json) proofs. It is not a single 379-rule execution. The full collector exited 1 because 51 rules failed at that source snapshot. [Its raw receipts](raw-receipts.tar.gz) and each supplement's linked proof and archive retain exact registrations, inputs, native observations, and child streams.

In the complete base run, the 51 failures comprised 30 missing implementations, 19 verifier failures, and 2 missing or ambiguous settings registrations (SIM222, SIM223). Passing base rows executed 8,837 case pairs: 8,753 raw exact pairs, 80 independently documented reference differences, and 4 execution-metadata asymmetries. The supplements add 540 intended-passing case pairs across seventeen rules; they do not rewrite failed base receipts. Failed rows retain their error details in the base summary and archive.

| Identity | SHA-256 |
|---|---|
| Pinned Ruff 0.16.5 executable | `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8` |
| Sealed Python worker | `d43a5760d6c9388d7ba307416417268fbf1cacb92ae6fbe9bf70dffdf0d686b8` |
| Corrected shared collector | `813f50ad3cf909b23a48da6801a50d756c349ab47129e7dd07815c6e277dd004` |
| Summary JSON | `a3eae09e90dcb4a2d7a88bb7e76fddc9888b31648133838fcb1edee3b11e6f91` |
| Raw receipts archive | `3f36a818e9addd6be75462f011656ddb3193c4c0b0f5b749bee84d74383b0bdc` |

The later [6bff CI inventory and speed receipts](../ci-6bff-full-inventory-2026-11-02/README.md) are a separate complete run at an older source head: 361 PASS / 52 FAIL, including 22 timeouts. They do not replace this source-bound combined proof.

The runner initialized all 413 rows as FAIL, checked each registration in pinned order, recorded each result atomically, and reached `run_state=complete`. Its summary binds the inventory and phase registry hashes; each passing row binds its query, cases, settings, imported modules, and native receipt digest. The worker used the default execution strategy. This single run establishes fixture results for this source and environment; it does not establish performance or cross-platform parity.
