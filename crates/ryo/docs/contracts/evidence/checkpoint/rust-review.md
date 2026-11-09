# Rust gate: checkpoint 332, fourth pass

- Reviewed source: `eedb4f1a77317542e3db0ccb5e3fe3ebd4f26cda`.
- Delta: `2991a41cd2..eedb4f1a77`. The third pass, at `2991a41cd2`, FAILed on F5 to F8.
- The delta carries four things:
  - the t611 edit-run fix (`c2ba5669a6` and `bec665436a`, merged at `2c94c8e3d5`);
  - the tm import-reading repair (`db77cdd326` and `96b5545cb8`, merged at `69caa24a4e`);
  - the `lookups.rs` split (`5b9555b1a0`);
  - the record (`eedb4f1a77`).
- Evidence read: the diffs, the files they touch, the branch leak re-reviews, and the integrator's logs in the validation332 directory. Under the review definition I ran no cargo.
- Tree: HEAD stayed at `eedb4f1a77` for the whole pass. The only working-tree changes are this file and its JSON.

**Verdict: FAIL.** Items 3 and 6 fail, both on missing evidence; the code is sound. Items 1, 2, 4, 5 and 7 pass.

| Item | Verdict |
| --- | --- |
| 1 Ownership and API shape | PASS |
| 2 Compiler over tests | PASS |
| 3 Hot path | FAIL (F5: evidence only) |
| 4 One representation per concept | PASS |
| 5 Standard crate over home-grown | PASS |
| 6 Tests | FAIL (F8: evidence only) |
| 7 Rust fit | PASS |

## 1 Ownership and API shape: PASS

- **C1 is applied.** `crates/tastemaker/cli/Cargo.toml:35-46` and `:63-66` require the ryo SDK and the ryo language crates at 0.1.2, the same as core.
- **Tastemaker types** (`crates/tastemaker/crates/core/src/domain/imports/types.rs`):
  - `Name.binding` is now `Option<ModuleName>` (`:77`).
  - `Scope` is `Copy` and no longer carries its own path (`:84-90`).
  - Scopes are owned once per file, in `BTreeMap<RepoPath, Vec<Scope>>` (`:139`).
- `imports()` borrows the scopes per source and clones nothing (`crates/tastemaker/crates/core/src/domain/import_rows/service.rs:213-235`).
- **t611** passes the file's number down by value (`crates/ryo/crates/core/src/domain/handing/service.rs:155,231,241`; `recording/service.rs:238,245`).
- `sdk/api.txt` is unchanged.

## 2 Compiler over tests: PASS

- **The memory guard can no longer pass vacuously** (`crates/tastemaker/cli/tests/replay/check.rs:328-376`).
  - It now requires exit 1 and a JSON report with at least one finding before it reads the peak.
  - Before, it accepted exit codes 0 to 2. 96b5545cb8 shows a pre-branch tm refusing with exit 2 after 8.6 s at a 454 MB peak, so the 331 "pass" measured nothing.
  - The harness itself was repaired; no parallel test was added.
- **t611 keeps one numbering per file.** A part read of the file itself is stored at `PartialAt(0)` and given the reading run's number when read back (`handing/service.rs:241-260`), the same convention `moved_words` uses.

## 3 Hot path: FAIL (F5, evidence only)

**The import reading's own cost is now proportionate.** Each part of my third-pass finding is answered:
- **The query.** `SCOPES` (`crates/tastemaker/crates/core/src/domain/import_rows/constants.rs:40-47`) selects by kind equality. It emits only the `stmt-list` that is the parent of a `use`, joined through `syntax-child`, plus the `item-list` nodes of inline modules. Before, it emitted every block of every file.
  - The leak re-review's probe table supports "nothing is missed": a `use` stands only in a `StmtList`, and the module test reads item lists only.
- **The rows.** Scopes are grouped per file with no path per row.
- **The scans.** Each import's innermost scope is computed once (`service.rs:231-235`). The innermost module around an edge is found by bisection over the file's few scopes (`innermost`, `:144-157`).
- **The recorded peak** went from 4,491,136 to 4,402,760 KiB (`tastemaker_workspace_69caa24a4e.log:7591`).

**Is landing with the engine's retention as a named, owned red acceptable under item 3? Yes in principle.**
- Item 3 judges the diff, and the retention is not in this diff.
- The guard was vacuous before, so the 331 "green" said nothing. A red guard here shows a real problem, now made visible, not a regression this diff introduced.
- It has an owner ("Project phase retains 2 GB of rows on a cold tm check of crates/ryo"), and the test stays red instead of being ignored. That is the right state for a known red.

**Why item 3 still fails: nothing committed supports the attribution.** Missing evidence for an item is FAIL.
- The two numbers it rests on are in no file in the validation332 directory, in no record line and in no evidence file:
  - the pre-branch tm peak of 4.34 GB, measured on a run that actually checked the tree;
  - the heap profile putting 2.04 GB in project-phase row retention.
- The README's Tastemaker workspace row states the attribution without a number behind it.
- The only recorded runs without the branch are the vacuous ones:
  - `gatefix_tastemaker_without_tm_branch.log:14091` ("ok" from an exit-2 run);
  - `gatefix_tastemaker_at_331.log:47`.
- So, from recorded data alone, it cannot be ruled out that this diff still adds most of the 4.40 GB.

Required change, evidence only, no code:
1. Put two logs in the validation directory, beside `tastemaker_workspace_69caa24a4e.log`:
   - **The pre-branch peak.** Build tm from the pre-branch core, `f67311c990`'s `crates/tastemaker`, with the provider-imports proof made to pass so the run checks the tree. Run the non-vacuous guard command (`tm check --format json` over the scratch copy of `crates/ryo`, under `/usr/bin/time -f "peak-kib %M"`) and keep its exit code, its finding count and its peak.
   - **The heap profile summary** of the current tm on the same run: the top allocation sites with their retained bytes, at least the project-phase row-retention frame and the `import_rows` frames.
2. Cite both in the README row and in `validation.json` `tastemaker_workspace`.

If the pre-branch peak is within about 100 MB of 4.40 GB and the import_rows frames are small, item 3 passes with the guard as a named, owned red.

## 4 One representation per concept: PASS

- `Scope` no longer duplicates the file path.
- t611 removes a second numbering: before, a stored file number from another run was read back as this run's (`bec665436a`).
- The `lookups.rs` split moves fixture cases into `fixtures/stores.rs` without copying them.

## 5 Standard crate over home-grown: PASS

No dependency is added. The only manifest change raises version requirements from 0.1.1 to 0.1.2.

## 6 Tests: FAIL (F8, evidence only)

**F6 is repaired.**
- `keep()` keeps every `static-import` row with a target, with `binding` optional (`crates/tastemaker/crates/core/src/domain/import_rows/service.rs:73-80`).
- The case `an_import_that_binds_no_name_is_kept_and_a_path_is_never_its` pins it.
- Core reporting is green: `f8_green_core.log` shows 4 passed, 0 failed.
- The Tastemaker workspace is 264/4 (`tastemaker_workspace_69caa24a4e.log`). The 4 are the memory guard (F5) and the 3 tests already red at 331.

**F7 is repaired.**
- The two t611 fixes name their tests, both rows of the core lint `lifecycle` group.
- Their red-then-green is machine-recorded: `t611_red_*.log` and `t611_green_*.log` cover lifecycle, samples, the go/api repro and the core library. These are `red_green` entries 31-34.

**F8 is not proven.**
- `validation.json` `red_green` entries 35-43 list 9 reds. Only one is in an integrator log: the reporting test, red at `0f2fa35a12` (`gatefix_tastemaker_workspace.log:7960`).
- The 8 `import_rows` reds are copied from the message of builder commit `96b5545cb8`. That message says they were seen with "the case runner made to report every miss", a temporary local change.
- The leak re-review of `96b5545cb8` ran none of them.
- The validation directory holds `f8_green_core.log` and no red log.
- So a claim stands in for a run. Item 6 requires that the integrator's red-then-green record prove the named test.

Required change:
1. On the merged head, reverse each fix's production hunk in turn:
   - `e2c66a218a`'s `in_scope` gating (3 rows);
   - `55e5fd3520`'s use-site drop (4 rows);
   - the optional-binding keep (1 row).
2. Run `tastemaker-core` `domain::import_rows::service_test::succeeds` after each revert. Use the standard case runner, which already lists every failing row, or a committed per-row option, not a local edit.
3. Save each red log, restore the head, and save the green log.
4. Name the logs in the 8 entries.

`a_path_at_a_use_site_from_the_package_root_is_an_import_of_its_own` is green in every revert by design: it guards against dropping too much. That is correct.

## 7 Rust fit: PASS

- The delta adds no `unsafe`, no `#[allow]`, and no `unwrap`, `expect` or `panic` in production code.
- `innermost` returns `None` for an unplaced span (`import_rows/service.rs:144-147`).
- `bindings` skips imports that bind no name with `filter_map` and `?` (`:231-235`).
- t611's `u32::try_from(place.0).unwrap_or(u32::MAX)` (`crates/ryo/crates/core/src/application/file_analysis/streaming/service.rs:316`) follows the existing pattern of that loop.

## Advisories

No bead ids here. Each advisory becomes a bead, or is dropped with a reason.

- **D1. `innermost` scans backward linearly.** After the bisection it walks back through the earlier spans with `rev().find(..)` (`import_rows/service.rs:149-156`). For a span that no scope holds, it visits every earlier sibling. With only blocks holding a `use` plus inline modules this is small today. Fix: keep a nesting stack from the sorted spans, or stop at the first span that ends before the target starts and has no enclosing parent.
- **D2. The scopes map takes an owned path per row.** `rows.scopes.entry(path)` (`import_rows/service.rs:107-110`) is handed an owned `RepoPath` for every scope row and drops it when the file is already present. Fix: look the file up first with `get_mut(&path)`, and insert only when it is absent.
