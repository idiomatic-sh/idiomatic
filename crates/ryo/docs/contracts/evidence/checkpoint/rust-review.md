# Rust gate: checkpoint 332, sixth pass

- Reviewed source: `110b66de9e9feda9ccd0060ad76824b64767c299`.
- Delta: `5e56d8e918..110b66de9e`. Pass 5, at `5e56d8e918`, FAILed on item 6 (F9).
- The delta carries:
  - the F9 repair to the memory guard (`1123506752`);
  - E1, which collapses the F8 entries;
  - the record (`110b66de9e`).
- Evidence read: the diff, `crates/tastemaker/cli/tests/fixtures/replay/fixes.rs`, `validation.json`, and `f9_red_memory_guard.log` in the validation332 directory and `f9_green_memory_guard.log`. Under the review definition I ran no cargo.
- Tree: HEAD stayed at `110b66de9e` for the whole pass.

**Verdict: PASS, 7 of 7. Checkpoint 332 may land.**

| Item | Verdict |
| --- | --- |
| 1 Ownership and API shape | PASS |
| 2 Compiler over tests | PASS |
| 3 Hot path | PASS |
| 4 One representation per concept | PASS |
| 5 Standard crate over home-grown | PASS |
| 6 Tests | PASS (F9 repaired, E1 applied) |
| 7 Rust fit | PASS |

## 1 Ownership and API shape: PASS

The delta changes no production code. Pass 5's reading stands:
- `get_mut` fills the scopes map (`crates/tastemaker/crates/core/src/domain/import_rows/service.rs:107-113`).
- The scope selection takes an `Option<ScopeKind>` (`:216`).

## 2 Compiler over tests: PASS

The guard uses the shared exit codes `CLEAN` and `FOUND` (`crates/tastemaker/cli/tests/fixtures/replay/fixes.rs:26`), not literals (`check.rs:3`, `:364-367`).

## 3 Hot path: PASS

As at pass 5:
- The import reading costs +58,344 KiB over the pre-branch tm on the same archive (`memory-guard-pre-branch.txt`).
- At the peak, 70.1% of live bytes are project-phase rows (`memory-guard-heap.txt`).

The memory guard now fails only on its subject: 4,452,912 KiB against 1,331,200 KiB. It is a named, owned red ("Project phase retains 2 GB of rows on a cold tm check of crates/ryo"). The retention predates this diff.

## 4 One representation per concept: PASS

No change.

## 5 Standard crate over home-grown: PASS

No change.

## 6 Tests: PASS

**F9 is repaired** (`crates/tastemaker/cli/tests/replay/check.rs:360-376`).
- The guard accepts exit 0 or 1.
- It still refuses exit 2 and a report it cannot parse.
- Before it reads the peak, it requires `findings + suppressed > 0`. A clean tree, which carries 13 in-source suppressions, is measured; a run that checked nothing is refused.
- The comment states the rule the code enforces.

**Red then green, machine-recorded on `1123506752`:**
- **Red, the old guard:** `f9_red_memory_guard.log:7326-7330` panics at `check.rs:363`: "tm check did not end with the tree's findings", left `Some(0)`, after a peak of 4,487,892 KiB. That is the false premise pass 5 found.
- **Green, the new guard:** `f9_green_memory_guard.log:7325-7326` gets past every precondition and fails at the peak assertion, `check.rs:381`: "reached 4452912 KiB, over the guard of 1331200 KiB".

For this test, "green" means it now fails on its subject instead of its premise. That is the right state for a named, owned red, and the test is not ignored.

**E1 is applied.** F8 is now three `red_green` entries (35-37), one per reversal. Each names `domain::import_rows::service_test::succeeds` with its own red log and `f8_green_restored.log`. That is exactly what the logs prove.

## 7 Rust fit: PASS

- The delta adds no `unsafe`, no `#[allow]`, and no `unwrap` or `expect` in production code.
- The guard's panics are test assertions whose messages say what to read.

## Owned reds that land with 332

Neither is a gate finding.
- The `crates/ryo` cold memory guard (4.45 GB against 1.33 GB), owned by "Project phase retains 2 GB of rows on a cold tm check of crates/ryo".
- The reds listed in `validation.json`, each with its owner, unchanged by this delta.
