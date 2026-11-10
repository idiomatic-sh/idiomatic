# Leak review: claude/w13-bench-parity (fourth pass)

Source `f44f7bc4617ec1c9a6bd5f136fae082dad4ee0cc`, base `1086daee4b`. This replaces the review of `bcb20a20c0` (FAIL: 2). The new commit changes only `application/measurement/service_test.rs` (5 lines). Read from `git diff`; no build was needed.

## 1 Proof where the contract lives: PASS
Unchanged. The contracts are proven in the shared crate over fakes.

## 2 Every enforcement claim fires: PASS
`adapter_pin` (`application/measurement/service_test.rs:173-204`) now fails without a launch:
- It returns `Err` unless the execution fake recorded exactly one launch (`history_len(&execution) != Some(1)`, `:196-198`).
- It reads the launch environment with `ok_or_else` (`:199-200`).
- Both adapter-pin rows expect a `Value`, and the case runner (`crates/ryo/crates/benchmark/src/cases.rs:91-92`) fails a row that expects a `Value` and gets an `Err`.

A run that never launches therefore fails the untimed row as well as the timed one. The discarded `run_with` result (`:189`) is only the decode refusal of the fake output, which comes after the launch the row checks.

The other measurement rows cannot pass without a launch either:
- `plain` expects `history_len` 1 (`:120`).
- `pinned` propagates `run_with` with `?` and asserts that `LINTER_HOME` survives.

The parity repair from `ede06f3bb3` holds: outside-rule, syntax and skip disagreements count as unaccepted, and agreement cannot hide a finding difference. The REDIRECT hook bypass was fixed outside this branch (`3d852bc6d1`).

## 3 Boundaries hold across crates: PASS
`run_with`, `Pinning`, `compare_rules` and `position` remain `pub(crate)`. The planted cross-crate call at `ede06f3bb3` failed with 3 E0603 errors, and no later commit changes visibility.

## 4 One implementation and one representation per concept: PASS
There is one thread-pin name: the contracts crate's `THREAD_PIN`, used by both adapters, `Pinning::Untimed` and the Ruff scenario test. `WORKERS_ENV` is deleted.

## 5 Layer direction: PASS
Unchanged.

## 6 Test seams stay out of production: PASS
The test helpers are `cfg(test)`. No shipped product links the benchmark crate.

## Advisories (not scored)
- An accepted difference pins only side, file and start. `differing-rules` skips rules whose only difference is an end position.
- `Untimed` unpins only an environment variable. An adapter that pins threads by argument stays pinned, which affects speed only.
- `diagnostics::position` splits lines on `\r\n`, `\r` and `\n` only (JS U+2028/U+2029).

## Enforcement candidates
None. The previous candidate's class (a discarded run result read with `unwrap_or_default`) is fixed at its only instance.

VERDICT: PASS
