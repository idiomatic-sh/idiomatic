# Leak review: claude/cp334-d (checkpoint 334 gate-fix batch, builder D)

- Source: `bb1468438a0ba99e5ec46f09306e86e9b09b8a4f`. Base: `752cc02c32`.
- Scope: §2.7 (`.19`, the allocation runners report a case id once), plus the PM-approved addition: harness_case answers libtest's command line.
- Closure: `ryo_language`, `ryo_core` (its test target), `ryo_sdk` (`resources/languages/contract.rs`), and the ten language runners that call `allocations_of_linked`. No manifest changed.
- Reviewed tree the builder worktree: HEAD and `git status --porcelain` were the same before and after (bb1468438a, clean).
- Builds ran in a detached scratch worktree at the sha, with its own target dir, through the builder test form the guard allows. Both were deleted after the review.

## 1 Proof where the contract lives: PASS
`harness_case` and its seam tests live in `ryo_language` (`fixtures/samples.rs:184-248`, `samples_test.rs`), next to `allocations_of_fixtures`, which owns the allocation row. No proof depends on `benchmarks/` or on a product rule set.

## 2 Every enforcement claim fires: PASS
- **Failure stays a failure.**
  - A body that panics is reported as `FAILED`, then `resume_unwind` runs (`samples.rs:241`).
  - Planted red in `language/tests/allocation.rs:275` (`greedy_reader: true`): the runner printed `test allocation::language::reading_allocates_as_the_parser_library_does ... FAILED`, the `failures:` list and `test result: FAILED`, then exited 101.
- **Re-planted 4 of the 6 recorded bugs.** Each turned its named seam test red, and each was reverted:
  - `resume_unwind` dropped: `a_panicking_case_still_panics_after_its_report` and `a_filter_that_names_the_case_runs_it` went red (2).
  - `--list` runs the body: `a_listing_names_the_case_without_running_it` went red.
  - `--exact` matches every name: `a_filter_that_names_another_case_runs_nothing` went red.
  - `--ignored` not read: `a_listing_of_the_ignored_cases_names_none` went red.
- **The listing protocol matches libtest.** Checked directly on the freshly built language runner:
  - `--list` and `--list --format terse` (also `--format=terse`) print `<id>: test` and exit 0.
  - `--list --format terse --ignored` prints nothing.
  - `<id> --exact --nocapture` (nextest's run form) runs the case.
  - A substring filter runs the case. `--skip` with a substring, or with `--exact` and the full id, leaves the case out. `--skip lang --exact` still runs it.
  - `--include-ignored` and `--test-threads 1` run the case.
  - The options that take a value (`samples.rs:201`) are all of libtest's, so no option value is read as a filter.
  - No allocation target in `commands.json` (`default_features`, `core_allocation`) gets a filter. An accidental filter on the command line therefore cannot hide a red in the gate.
- **Python's red is FAILED by id.**
  - The python `allocation` binary exited 101 with `test allocation::python::reading_allocates_as_the_parser_library_does ... FAILED`, for the existing 40 blocks in 20 files.
  - Fed that output, run.py's `test_results` at the sha records a failed test with that id, not CRASHED.
  - Caveat: the guard only allows a builder-form test run of a package the branch changes, and python is not one. So this was the builder's binary, built at 04:34 (commit at 04:38). Its strings contain the listing code, and its `--list` answers correctly.
  - Rust's binary: `... ok`, exit 0.
- **Core's red is unchanged under the new id.** It is the same `measures.rs:115` assertion as before, now reported as `test allocation::core::reading_allocates_as_the_parser_library_does ... FAILED`. It was built fresh from the sha. The integrator's re-key of `run.py:289` and `run_test.py:172,178,250` is merge work under §2.7.
- **The ignore at `allocation.rs:35` is gone.**
  - no-silent-skip fires once on the base runner with its ignore stripped (`:35` `return`). So the rule is live, and the new runner no longer needs the ignore.
  - tm 0.1.2 on the five touched files: `0 error(s), 0 suppressed`.

## 3 Boundaries hold across crates: PASS
- `harness_case` is `pub` under `#[cfg(any(test, feature = "fixtures"))] pub mod fixtures` (`parse.rs:10-11`). `case_by_arguments` and `report` are private.
- The SDK caller is under `#[cfg(feature = "fixtures")] pub mod contract` (`languages.rs:3-4`).
- No new dependency edge.

## 4 One implementation per concept: PASS
- Core's hand-written cargo report is deleted.
- `git grep 'test result: '` over `*.rs` finds only `samples.rs` and its test.
- All 12 allocation runners report through the one function.
- The seam tests call the production `case_by_arguments`.

## 5 Layer direction: PASS
The callers import ryo_language's inbound fixtures, as before. No domain or application module imports the fixture.

## 6 Test seams stay out of production: PASS
- Every new item is behind the fixtures or test gate.
- `fixtures` is switched on only by dev-dependencies or by a crate's own `fixtures` feature (unchanged).
- So the change adds no production surface (criterion 20).

## Advisories (no FAIL)
- When a filter leaves the case out, the runner prints nothing. libtest prints `running 0 tests` and a `1 filtered out` summary. Both exit 0, and no gate command filters these targets, so no red is hidden.
- An unknown flag is ignored, where libtest exits 101. A typo therefore runs the case rather than skipping it, which errs toward running.

VERDICT: PASS
