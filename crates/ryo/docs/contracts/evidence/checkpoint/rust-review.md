# Rust gate: checkpoint 332, fifth pass

- Reviewed source: `5e56d8e9182c341a4c594831b0b628970b89a153`.
- Delta: `eedb4f1a77..5e56d8e918`. Pass 4, at `eedb4f1a77`, FAILed on F5 and F8, on evidence alone.
- The delta carries:
  - D2 and the bare-`bool` fix (`beb55b206a`);
  - the F5 readings, `memory-guard-pre-branch.txt` and `memory-guard-heap.txt`;
  - the F8 red logs;
  - the record (`5e56d8e918`).
- Evidence read: the diff, the committed readings, and the raw logs in the validation332 directory: `f5a_*`, `f5b_*`, `f8_red_*`, `f8_green_restored.log`. Under the review definition I ran no cargo.
- Tree: HEAD stayed at `5e56d8e918` for the whole pass.

**Verdict: FAIL.** F5 and F8 are repaired, but a new finding, F9, fails item 6. The other six items pass.

| Item | Verdict |
| --- | --- |
| 1 Ownership and API shape | PASS |
| 2 Compiler over tests | PASS |
| 3 Hot path | PASS (F5 repaired) |
| 4 One representation per concept | PASS |
| 5 Standard crate over home-grown | PASS |
| 6 Tests | FAIL (F9; F8 repaired) |
| 7 Rust fit | PASS |

## 1 Ownership and API shape: PASS

**D2 is applied.**
- `keep()` pushes onto a file's existing scopes through `get_mut`, and moves the path into the map only for a new file (`crates/tastemaker/crates/core/src/domain/import_rows/service.rs:107-113`).
- The scope selection now takes `Option<ScopeKind>` instead of `&dyn Fn(ScopeKind) -> bool` (`:216`, `:231-232`). The type states the meaning, and the dynamic call per scope is gone.

## 2 Compiler over tests: PASS

The delta changes no visibility. The compiler checks the `Option<ScopeKind>` filter; it could not check the bool predicate it replaces.

## 3 Hot path: PASS (F5 repaired)

### Condition (a): the pre-branch peak is within about 100 MB

Met. `memory-guard-pre-branch.txt` records both runs on the same git archive of `781653e221`:

| Run | Peak | Exit | Findings |
| --- | --- | --- | --- |
| pre-branch tm | 4,382,468 KiB | 1 | 1,253 |
| current tm | 4,440,812 KiB | 0 | 0 |

The difference is +58,344 KiB. The raw logs are `f5a_base_tm_check.log` and `f5a_current_tm_check.log`.

### Is the comparison like for like? Yes, where the peak is taken

- **Same engine work.**
  - Both binaries link this tree's ryo crates.
  - The base archive swaps in the provider-imports spec from before the branch. That spec differs from the current one only in its `[[scenarios]]` (`crates/ryo/tastemaker/specs/provider-imports/0.1.0/spec.toml`: 320 added lines, all scenarios, no query).
  - So the engine evaluates the same rules over the same files. The one difference is the built-in scopes rule, which is exactly the branch's cost.
- **The findings difference does not reach the peak.** The 1,253 findings are the pre-branch tm's use-site false positives. They are produced after the engine answers, while the peak sits inside the lint answer: 70.1% of live bytes is under `ryo_core` lint `answer → crossed → ordinary → evaluate_occurrences → row` (`memory-guard-heap.txt`).
- **If the findings matter at all, they inflate the base.** Extra findings can only add memory on the base side, so +58 MB, if anything, overstates the branch's cost. It cannot hide a larger one.
- **Limits.** Each side is one sample, taken at load above 3 (8.6 for the base run, 11.5 for the current one). Peak RSS is far less sensitive to load than wall time, and wall time is not used here. Advisory E2.

### Condition (b): the import_rows frames are small

Met, but the reading is weak on its own. `--focus=import_rows` finds 0 live bytes at the peak. `import_rows` runs after the lint returns, so that is mostly a matter of timing. The scopes query's own engine-side cost would sit under ryo frames, and condition (a)'s A/B delta captures it. So (a) carries the attribution.

### The guard as a named, owned red

Acceptable under item 3:
- the project-phase retention predates this diff;
- the old guard was blind to it;
- it has an owner ("Project phase retains 2 GB of rows on a cold tm check of crates/ryo").

The guard is now red for the wrong reason, which is F9 under item 6.

## 4 One representation per concept: PASS

Scopes are still one list per file.

## 5 Standard crate over home-grown: PASS

No dependency change.

## 6 Tests: FAIL

### F8 is repaired

Each fix's production hunk was reversed on the merged head `beb55b206a`, and the standard runner then turned `tastemaker-core` `domain::import_rows::service_test::succeeds` red:

| Fix reversed | Red log | Row the log names |
| --- | --- | --- |
| scope gating (`e2c66a218a`) | `f8_red_scope_gating.log` | `a_path_in_an_inline_module_sees_no_import_written_outside_it` |
| use-site drop (`55e5fd3520`) | `f8_red_use_site_drop.log` | `a_path_at_a_use_site_through_a_name_an_import_binds_is_that_imports` |
| optional-binding keep | `f8_red_optional_binding_keep.log` | `an_import_that_binds_no_name_is_kept_and_a_path_is_never_its` |

- `f8_green_restored.log` is green.
- So each fix is shown red then green on the test the record names.
- The runner stops at its first miss, so 5 of the 8 row entries (35-42) name a row no log shows. The README states this. Advisory E1.

### F9 (new): the memory guard's premise is false at the head

- **What the test asserts.** `checking_crates_ryo_cold_stays_under_its_memory_guard` checks exit 1 and at least one reported finding before it reads the peak (`crates/tastemaker/cli/tests/replay/check.rs:359-373`). Its comment says "crates/ryo has findings".
- **What the committed evidence shows.** The current tm on a git archive of this head ends with exit 0, 0 findings and 13 suppressed (`memory-guard-pre-branch.txt`, current run).
- **So, at this head:**
  - the guard fails on "tm check did not end with the tree's findings" before it ever reads the peak;
  - after the retention fix, with the tree kept clean as intended, it can never go green.
- **The record's "stays red at 4,402,760 KiB" is stale.** It is the reading at `69caa24a4e`, when `lookups.rs` at 409 lines still gave a file-length finding. The test has not been run since.
- **A correction.** In pass 4 I PASSed this guard under item 2 without checking its premise against a clean tree.

Required change:
1. Prove that the run checked the tree without requiring the tree to be dirty. Accept exit 0 or 1, parse the JSON report, and require `findings.len() + suppressed.len() > 0`. `crates/ryo` holds 13 in-source suppressions, which only a run that checked the tree reports. Better still, if the report carries a count of checked files or rules, require that it be non-zero.
2. Keep refusing exit 2 and a report it cannot parse.
3. Correct the comment at `check.rs:360-362`.
4. Rerun the test at the new head and record its reading. It should fail on the peak, about 4.4 GB against the 1.3 GB bound, and nowhere else.

## 7 Rust fit: PASS

The delta adds no `unsafe`, no `#[allow]`, and no `unwrap` or `expect`. The discarded result of `insert` is typed (`import_rows/service.rs:111`).

## Advisories

No bead ids here. Each advisory becomes a bead, or is dropped with a reason.

- **E1. Five F8 row entries rest on the builder's word.** Entries 35-42 name eight rows, but only three appear in a red log. Either name the test function with the log, the granularity the logs prove, or give the case runner a committed every-miss option so a red log lists every failing row.
- **E2. The F5 readings are single runs under load.** Each side is one run taken at load above 3. If the bound is ever argued in tens of MB, repeat both at load under 3 under the measure lock.
