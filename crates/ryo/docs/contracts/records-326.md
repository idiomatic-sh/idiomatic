# Integration records from checkpoint 326

The integration records of checkpoints 326 to 330 and the branch records they merged, newest checkpoint last; [records.md](records.md) holds checkpoints 319 to 325 (at its `file-length` limit), and the current checkpoint's record is in [the matrix](README.md).

## Checkpoint 326 integration record

Checkpoint 326 merges two branches into checkpoint 325 (`dcd94840fc`), each as a merge commit, in this order: review repairs X (`claude/w8-review-repairs-x` `600085d4c6`, merge `1579f3cd72`) and perf iteration 2 (`claude/p2-per-file-and-parallel` `b466ab2102`, merge `4b674e1e9f`). validation326 ran on `4b674e1e9f` (raw logs in `ryo-contract-validation326-20261108`).

- **Joins:** review repairs X merges without conflict. Perf iteration 2 is cut from 324 (`14a8352416`); of the files 325 also changed, `Cargo.lock` and the benchmark's local engine adapter merge by git without conflict (`cargo check --locked` holds the lock file), and its engine edits fall on paths 325 did not touch. One conflict, in this file: the branch added its "Perf iteration 2" row beside the ring row it carried from 324 (`crates/execution/src/domain`, 24 at validation323), where 325 had rewritten that row (`crates/execution_domain/src/domain`, 18 at validation324). Both rows are kept, the ring row in 325's words.
- **Lines under `crates/ryo` (the two merges against 325):** +327 / -146 over 40 files (Rust +174 / -116 over 29 files).
- **Matrix upkeep:** the 325 record moves to [records.md](records.md) so this file stays within `file-length`. validation.json's hand-kept notes are rebound to validation326 with the values the raw logs give (the cli replay warm peak, the `cli_cost` measures, `ruff_rule_phases` exit 0, the canary and ruff targets on the default pool).

| Pass | Command | Errors reported | Errors fixed |
|---|---|---|---|
| 1 | workspace check (all features, all targets) | 0: clean | 0 |

The pass is the workspace check with all features and all targets, `--keep-going` and JSON diagnostics, under the build lock (14.7 s); its log is in `check-passes/` of the validation directory. No warning falls on a line this checkpoint added. The full test build is validation326's first target.

- **validation326 (rows, from `tests.json` and the raw logs):** libraries 1079 / 2 (the same two named reds: core `lint::use_case_test::succeeds` row `a_removed_file_leaves_no_unknown_or_skip_behind_in_any_store`, benchmark `process_capture`); boundary 97 / 0; activation 11 / 0; bdd 7, core bdd 18; `ruff_sharing` 1 / 0; `ruff_canary` 1 / 0 (112, the split asserted); ruff 13 / 0; doctests 33 / 0; conformance exit 0; `ruff_rule_phases` exit 0 (red from 321 to 325, repaired by review repairs X); cli replay 75 / 3 (the same three reds; the store of ruff 78,525,063 bytes as at 325, warm peak 113,549,312 bytes against 113,012,736); `cli_cost` 1 / 3 with 3 not run (cold 9,539 ms, CPU 55,653 ms, peak 585 MB, warm 521 ms, edit 2,659 ms); python library 138 / 0; language 43 / 0; hooks 45 / 0; `checkpoint_check` 15 / 0; `no_print` exit 0; `bench_build` exit 0; `core_allocation` exit 101 (Phase 7); `tm_source` 0 findings, 2 suppressed; `tm_validate_all` exit 0.
- **Determinism at default threads:** perf iteration 2 runs the benchmark worker's files on rayon's own pool size (`RAYON_NUM_THREADS` when set, else one thread per core; one worker at 325). validation326 sets no thread count, so the canary and the `ruff` engine tests that go through the benchmark's local engine ran on 10 threads here, and both answer as recorded at one worker (112; 13 / 0). The full Django receipt below gives the same bytes at default threads and at one thread as the 325 receipt at one thread.
- **The settings identity of `RAYON_NUM_THREADS=1`:** perf iteration 2 adds `RAYON_NUM_THREADS=1` to the environment of the harness's ryo invocation, so a bake-off compares one thread with one thread. No stored evidence pins the candidate's settings. Every stored settings identity is the reference (Ruff) process, which already carried `RAYON_NUM_THREADS=1`: the 13 `results/discrepancy-profile-context-2026-11-05/*/reference.json`, the 8 `results/up031-current-context-2026-11-04/**/invocation.json`, B031's `uncertified-cases.jsonl` (its `discrepancy_validation.settings_sha256` is the reference certificate's on both reference and candidate rows; `require_settings` checks the reference variant only), and the `source_certificate_validation.settings_sha256` of B031's `registration.json`, FURB132's and RUF019's `contract.json` and UP008's `provenance.json`. No stored identity changes, and `ruff_rule_phases` (which replays the frozen evidence guards) exits 0.
- **Whole-tree TM:** 18 findings, 6 suppressed (the six admitted ignores), all `no-domain-concept-cycles` in `crates/execution_domain` (the ring row). File-level ownership cycles 0 over 4,864 edge occurrences (4,864 at 325).
- **Acceptance blocks on the merged tree** (without each block's worktree-state and branch-head lines): review repairs X exit 0 (method.md names the measured source and the digest; `run.py --check` exit 0; the record present; `tm` over `docs/contracts` 0; no ignore added). Perf iteration 2 exit 0 (`tm` over its changed `.rs` files at 326 0, `perf-p2.md` present, the README row present, no ignore or allow added). Both blocks also exit 0 in their own worktrees at the frozen heads.
- **Django receipt (full Django 413 cold, 2,875 files x 413 rules, fresh store, under the build lock):** the release `ruff` bench worker built from `4b674e1e9f` (sha256 `43cc4df31c688b82fa32a256506ec4500bf06ffb9e02f104a62857888185413f`). The request was staged by `verify_profiles.py --stage-request <the pinned 323 cold request> --source 4b674e1e9f --output-dir <fresh directory>`, which printed the staged digest `04f6c9d81a6c4a6f1c5c26395b68b33fa0b05eee81ae19ad997ab767e1f11ea1` (1,088 files; the digest of 325, since rules, fragments and `benchmark.toml` did not change) and request sha256 `16c25649658d1a1869f46082adf56cc666a492300557501c326624e282162474`.

  | Threads | Exit | unknown_or_unread | Diagnostics | Skips | Wall s | CPU s (user + sys) | CPU / wall | Peak RSS | Load at start |
  |---|---|---|---|---|---|---|---|---|---|
  | default (10 cores) | 0 | 0 | 8,804 | 395 | 502.3 | 4,062.8 (3,954.6 + 108.2) | 8.09 | 7.00 GB (6,995,544 KB) | 1.66 |
  | one (`RAYON_NUM_THREADS=1`) | 0 | 0 | 8,804 | 395 | 2,729.6 | 2,728.6 (2,645.5 + 83.2) | 1.00 | 6.00 GB (5,995,392 KB) | 1.60 (5-minute 5.77) |

  Coverage at both: selected 1,187,375, evaluated 1,186,980, skipped 395 (every skip at `tests/test_runner_apps/tagged/tests_syntax_error.py`, `syntax-rejected`); store 1,778,454,983 bytes. The output bytes (sha256 `53b163f4bda83afa...`, 992,711 bytes) are those of the 325 receipt. Against 325 at one thread (3,383.4 s, 6.61 GB): one thread is 19% faster in wall (perf iteration 2's per-file cuts) with 0.61 GB less peak; default threads is 6.7 times faster in wall for 1.20 times the CPU, with 0.39 GB more peak than 325 at one thread (1.00 GB more than one thread here).
- **Landing 1 follow-ups (unchanged here):** `missing_debug_implementations` 109 sites across the workspace (68 in the three engine crates), never-used items 65 (execution 28, store 35, derived 2); they stay with their row.
- **The 322 hold:** lifted; the decision and its words, quoted, are under "Owner decisions".
- **Principal Rust review:** PASS on all seven items, first pass (diff `dcd94840fc..57f25dd99b`, `evidence/checkpoint/rust-review.json`). Its four notes (not failures) are left for the landing 1 follow-ups row rather than taking another check pass: check `unnecessary_lazy_evaluations` on the lazy refusals over fieldless variants (if clippy reports it, keep the change with a reasoned `#[expect]` citing the drop-glue measurement); trailing whitespace at `kernel_session/evaluation/service.rs` lines 228 and 251; `executable_program/value.rs:118` still builds `OperatingError::NotRead` eagerly; `CompletionPolicy::for_rule` repeats `rebind`'s scope match.

**Cycle metrics:**

| Metric | Checkpoint 326 |
|---|---|
| Builders | 2 (review repairs X, perf iteration 2) |
| BLOCKED questions | 0 |
| Time dispatch to FROZEN (first and last branch commit) | review repairs X 07:49 to 07:54, perf iteration 2 06:26 to 08:04 (2026-11-08 UTC; bead created 07:41 and 05:46) |
| Tokens per builder | not supplied to the integrator |
| Compile passes | 1 check pass (errors 0) and the validation's test build |
| Rows ticked by machine | 0 (`run.py --check` un-ticks no row) |
| Lines under `crates/ryo` | +327 / -146 (Rust +174 / -116) |
| Full Django 413 cold | exit 0, unknown 0; default threads 502.3 s, 7.00 GB; one thread 2,729.6 s, 6.00 GB (3,383.4 s, 6.61 GB at 325) |
| Contract review | pending |
| Rust gate review | PASS on all seven items, first pass (diff `dcd94840fc..57f25dd99b`) |

## Review repairs X

Branch `claude/w8-review-repairs-x` from checkpoint 325 `dcd94840fc`, for the contract review of 325 (BLOCKING on Phase 7, NOTE on Phases 1 to 6). Checked: `verify_profiles.py --self-test` exit 0; `run.py --check` exit 0; `tm` over `docs/contracts` and over the changed benchmark Python files 0. No other test was run, no engine or rule changed.

- **R1, request staging (the Phase 7 blocker):** method.md, "Measurement requests": every perf and receipt request stages its rules, query fragments and `benchmark.toml` from the measured source tree, never a `/tmp` copy, and its receipt records the staged digest beside the worker's sha256. The one staging command is `verify_profiles.py --stage-request <pinned request> --source <measured commit> --output-dir <fresh directory>` (`0af327c8f7`): `git archive` of the three inputs at the commit, every rule-side request field read from the staged `benchmark.toml`, the corpus side from the pinned request; it writes `request.json` and `staged.json` and prints the digest. The acceptance row cites it in place of "the committed runbook", which did not exist.
  - The 325 Django receipt, recomputed: `--stage-request` over the pinned cold request with `--source 316497ef75` stages 1,088 files whose digest is `04f6c9d81a6c4a6f1c5c26395b68b33fa0b05eee81ae19ad997ab767e1f11ea1`, the digest of the receipt's own hand-staged profile; the request it builds equals the receipt's request but for the rules path. Worker sha256 `afd4d2ab6ebff7d655043426c0bd527f128f996776f83cb56a77aeb9cc632224`. The Phase 1 headline names both.
  - Open under the rule: the `cli_cost` Django and one-file rows cut their request from the pinned cold request, whose `rules` are the 2026-11-05 copy. They were not available at validation325 (no worker named); before they measure they take a staged request. Owner: "Acceptance: 413/413 on the current engine, full Django (2,875 files, all 413 rules) with unknown 0 and unread 0, sub-second cold wall with low peak RSS, warm and edit faster, Rust regression guards within bounds."
- **R2, `ruff_rule_phases` (`1fb8543ef2`):** both causes of the red since 321 were in the harness. The staging guard compared the runner's command with the bare mock command, but since 2026-11-04 the verifier appends the case-receipt argument; the guard now expects it. The frozen-input guard rebuilt the 232 moved candidates of the 2026-11-02 normalization from the live `query.ryoql`, whose bytes have moved on (265 hash mismatches); it now reads each old path at the normalization's recorded source commit. The self-test exits 0; the target is green at the next validation.
- **R3, owner titles and stale text:** `run.py` keeps the owner mapping (`OWNERS`, each red to its README row by exact title) and writes the `owned by "<title>"` clause into each owned note on every run; `--check` fails when a mapped title is no README row or a cli replay red has no owner. validation.json now names "Close full TM, contracts, independent review and normal CI." for `ruff_rule_phases` and `process_capture`, "Repair every deferred output failure; rerun all 413 and full corpus." for the cache red and `removed_file`, and the kernels, frontier-decode and acceptance rows for the feed, warm peak and `cli_cost`. README:9 and the Phase 1 row describe validation325 (ruff 13/0, the Django receipt exit 0).
- **The 324 findings standing at 325, each closed here or with its owner:**
  - P1 owner titles: closed here (R3).
  - P1 row text: closed here (README:9 and the Phase 1 row).
  - P2 sparse activation row: closed here; the row cites `activation` 11/0 at validation325 and the deleted negative control's replacement (none: cargo-mutants after the alpha).
  - P2 rows "Seal every runtime/admission proof issuer", "Bind admitted proof", "Complete batch release", "Require complete declared root submission" and "Close scheduling-owned cycles/TM findings": rebound here to validation325 and the landing 1 table; each stays unticked for the independent re-review of Phase 2.
  - P3 actual413 rebind: closed here (validation325); the tick waits for an independent review of the row.
  - P4 "Require admitted acquisition across all 9 derived kinds": closed here; the row names what replaces the deleted census.
  - P7 pass log location: closed here; the check-pass logs of 324 (18 passes) and 325 (1 pass) sit in `check-passes/` of the validation directories `ryo-contract-validation324-20261108` and `ryo-contract-validation325-20261108`.
  - P7 the owner's hold: the owner's, in "Owner decisions pending"; the row now says 323 to 325 pass and the hold stands until the owner decides.
- **New at 325:** crates.md:12 says 9 kinds (closed here). The fixtures-gated `ryo_execution::inbound::use_cases::execution::fixtures::derived()` is the same port on the same owner route as `ryo_execution::derived()` and is named here, not folded (no engine change). The ratchet's one-file best 6,871 ms stays unexercised until `cli_cost` runs with a worker: the acceptance row.

## Review repairs IX

Branch `claude/w7-review-repairs-ix` from checkpoint 324 `14a8352416`, for the contract review of 324 (BLOCKING on Phases 1, 4 and 7, NOTE on 2, 3, 5 and 6). Checked: `tm` over the changed files 0; whole-tree `tm check` 18 findings (the ring), 6 suppressed, as at 324; `tm validate` holds the three new rule versions; `run.py --check` exit 0; the package-scoped `cargo check` of core, the SDK, the command line and execution (all features, all targets) is clean. No test was run and no behaviour changed.

- **R1, the engine-crate boundary (Phase 4 / hardening step 2):**
  - (a) Core, the SDK and the command line no longer depend on `ryo-execution-store`: their three manifests drop it. Core holds the store's owner-issued `DerivedAccess` rather than the `DerivedPort` itself. The prepare interface forwards `DerivedAccess` (`no-reexports` 0.5.0 admits it: an application type of an internal crate that `PreparedExecutionProgram` names). The edges compose the derived port through `ryo_execution::derived`, a crate-root composition that builds the same `use_case(semantics())` the edges built.
    - Constructors: `PrepareUseCaseImpl::new`, core's `Execution::new` and `LintUseCaseImpl::new` take `impl Into<DerivedAccess>`.
    - Decision: the crate root is the only home the placed rules admit. An execution `outbound/derived` factory is refused by `nu-hex/port-module` (no interface or fixtures) and by `factory-calls`. A `prepare/composition.rs` is refused by `use-case#closed` and `import-directions`. Overrule path: a placed composition role for an engine crate.
  - (b) `internal-package-allowlist` 0.2.0 (query unchanged; it already matches by `package =`, or by the key when there is none) is placed on every member manifest that lacked it: cli, benchmark, derived, query_language, values, `languages/*` and python. Each manifest is allowed exactly the internal packages it uses. New scenarios: `renamed-path-engine` (the review's seed) and `key-names-the-engine`.
  - (c) `library-surface` 0.2.0 reads `extern crate X as Y` as an import of `X`, so an alias of the library is reported where it is declared. `use X as Y` at the root already named `X`. New scenarios: `an-extern-crate-alias` (the review's seed), `a-root-use-alias` and `an-extern-crate-of-another-crate`. Its surface over ryo is now empty. 0.1.0 stays for the Tastemaker product's pin.
  - Seeds, reproduced on a scratch `git archive` copy of this branch:
    - `dom = { package = "idiomatic-sh-ryo-execution-domain", path = ... }` in `cli/Cargo.toml` with `use dom::...::compile` in `cli/src/edge.rs`: 1 `internal-package-allowlist` finding at `cli/Cargo.toml` (0 at 324).
    - `extern crate ryo_execution_store as st; use st::...::open_program_owner` in `core/src/application/engine/types.rs`: 1 `library-surface` finding at the `extern crate` line (0 at 324).
    - The same `extern crate` alias in `cli/src/languages.rs`: 1 finding.
- **R2, the Phase 7 rows:**
  - The ring row checks `crates/execution_domain/src/domain` and states 18 at validation324, with the 15-concept and 3-concept cycles named.
  - Hardening steps 1 and 2 state their 324 result and the exits still unmet.
  - The 324 record's counts state their definitions and give both: src only, and with `tests/`. The store and execution line counts are recomputed by that definition.
  - `no_print` clippies `idiomatic-sh-ryo-execution-domain` and `idiomatic-sh-ryo-execution-store` too (`commands.json`; it takes effect at the next validation).
  - The cadence row states the target and why 324 took 18 passes.
- **R3, the Phase 1 row:** the full Django result at 323 (exit 1, 11,496 unknowns = PT020/PT025/PT026/RUF100 x 2,874, the log path), with this row as owner and the second-pass fix in progress. It replaces the stale Django256 / 2,880 headline.
- **R4:** `fixtures-only-in-tests` 0.3.0 reports a gated method named by its path (`Trait::open_file(&x, ..)`, `Type::open_file(..)`) under the method seal. A path counts when its qualifier ends in a capitalised name, so `crate::open_file` stays a function. New scenarios: `production-calls-a-gated-method-by-its-path` and `a-path-named-like-a-production-function`. Both of the review's path seeds are reported on the scratch copy, and the whole tree is unchanged.
- **The 324 review's findings, each repaired here or with its owner row:**
  - P1 RUF100 pin and I001 (red): "Repair every deferred output failure; rerun all 413 and full corpus" (standing).
  - P1 Django unknowns: repaired here (R3).
  - P1 owned reds: the ruff I001 rows, removed_file and the cli cache size belong to "Repair every deferred output failure"; `ruff_rule_phases` belongs to "Close full TM, contracts, independent review and normal CI". `validation.json`'s notes take these titles at the next `run.py` run, which writes them. The reds stay red.
  - P1 row text bound to validation323: rebound at the next validation upkeep (integrator).
  - P2 `open_file` seal: repaired at 324.
  - P2 method-path form: repaired here (R4).
  - P2 the sparse-activation row's deleted control: "Require complete declared root submission; close sparse-work and all-route negative controls" (standing). The negative half waits for cargo-mutants after the alpha.
  - P2 rows 200 to 204 text: their rows.
  - P2 scheduler owner, note: "Contract hardening step 3".
  - P3 actual413 sharing: citations repaired at 324; the tick waits for an independent review of the row.
  - P3 ready-gate deleted controls, count every physical instruction, walk B: their rows (standing).
  - P4 the library boundary: repaired here (R1).
  - P4 the deleted derived census in "Require admitted acquisition across all 9 derived kinds": that row. What replaces the census is `derived-view-acquisition` (B3, B4), the exhaustive `DerivedKind` match and `join::family::{lifecycle,identity}`.
  - P4 rows 210 and 211 text: their rows (standing).
  - P5 canary split: repaired at 324. P5 cli replay feed: the kernels row. P5 fusion rows text: their rows (standing).
  - P6 warm peak: red, "Decode lazy packed frontiers once per population, not once per root". P6 rows 214 and 216: their rows (standing).
  - P7 receipts: `no_print` scope repaired here (R2).
  - P7 ring row: repaired here (R2).
  - P7 hardening rows 230 and 231, and the counts label: repaired here (R2).
  - P7 not run is not passed: repaired at 324.
  - P7 tm validate log: repaired at 324. Note: the `crates/ryo` run reuses kept proofs.
  - P7 report card: verified.
  - P7 cadence target and cause: repaired here (R2). The pass logs beside the validation logs: "Compile cadence: every error of a pass fixed in that pass" (integrator).
  - P7 review README upkeep, deletions 320 to 321, inventory 8 to 9: "Close full TM, contracts, independent review and normal CI" and "One implementation per kind for every registry, or a named port" (standing).
  - P7 the owner's hold on the push for the 322 FAIL: the owner's row under "Owner decisions pending" (323 and 324 are PASS; restating or closing it is the owner's).

## Checkpoint 327 integration record

Checkpoint 327 merges two branches into checkpoint 326 (`f3a9a7e95c`), each as a merge commit, in this order: review repairs XI (`claude/w9-review-repairs-xi` `be4dafcb5f`, merge `3b7e7ca1f4`) and perf iteration 3 (`claude/p3-fewer-polls` `7ae230667f`, merge `6b316e8b28`), then one integration fix (`142fe40a98`). validation327 ran on `142fe40a98` (raw logs in `ryo-contract-validation327-20261108`).

- **Joins:** review repairs XI merges without conflict. Perf iteration 3 is cut from perf iteration 2's head (`b466ab2102`). One conflict, in this file: the branch added its "Perf iteration 3" row after the perf iteration 2 row, where XI had rewritten that row (`relation_partitions` recorded as an index). XI's perf iteration 2 row, then the perf iteration 3 row, then the ring row in 326's words.
- **Integration fix (`142fe40a98`, the 327 brief item):** XI's `cli_cost` workload held the staged directory as a constant (the 326 receipt directory under the system temporary directory) in `cli/tests/fixtures/replay/workload.rs`. The constant is gone: the staged run is the directory of the worker that `RYO_PREFLIGHT_WORKER` (the one existing input) names, its `request.json` and the `staged.json` receipt beside it, so `inputs_at` refuses the request without a staged digest as XI wrote it. The worker now runs in the corpus root the request names (`root`); before, it ran in `<staged dir>/corpus`, which a `--stage-request` output directory does not have. No flag or variable added.
- **The new `cli_cost` cases:** `an_absent_input_is_a_typed_reason_not_a_pass` passes in validation327. With `RYO_PREFLIGHT_WORKER` naming the receipt worker in the staged receipt directory below, the N999 and one-file rows run end to end on the staged request: N999 over Django wall 10,401 ms and CPU 10,370 ms hold (best 10,297 / 10,294), peak 1,056 MB against best 979 (red; perf iteration 3 measures the same 1,056 MB before its change, so not this checkpoint's); one file x 413 wall 3,429 ms and peak 860 MB beat best 6,871 ms / 1,139 MB by more than 5%, which the row reports as "lower `best`". `ratchet.toml` is not changed here: its bests are seeded under the Phase 7 acceptance row with the ruff rows. The full-Django cold-and-warm row (two one-thread runs) was not run.
- **`core_allocation` owner:** the perf iteration 2 row, as XI chose; kept. Corrected by review repairs XII: the row "Measure full-Django cold/warm/edit and Rust regressions; meet final objective.", which decides its block limit.
- **Lines under `crates/ryo` (the two merges and the fix against 326):** +222 / -85 over 24 files (Rust +151 / -70 over 17 files).
- **Matrix upkeep:** the 326 record and the review repairs X and IX records move to [records-326.md](records-326.md) (records.md is at its `file-length` limit). validation.json's hand-kept notes are rebound to validation327 with the values the raw logs give.

| Pass | Command | Errors reported | Errors fixed |
|---|---|---|---|
| 1 | workspace check (all features, all targets) | 0: clean | 0 |

The pass is the workspace check with all features and all targets, `--keep-going` and JSON diagnostics, under the build lock (44 s); its log is in `check-passes/` of the validation directory. No warning falls on a line this checkpoint added. The full test build is validation327's first target.

- **validation327 (rows, from `tests.json` and the raw logs):** libraries 1078 / 3: the two named reds of 326 (core `a_removed_file_leaves_no_unknown_or_skip_behind_in_any_store`, benchmark `process_capture`) and, in this run only, benchmark `direct::adapter_test::succeeds` (row "missing script interpreter is a launch failure" answered `ExecutableFileBusy` for `NotFound`: the ETXTBSY race of a script executed while another test thread holds its write descriptor; no benchmark file changed since 326; five reruns of the benchmark library under the lock give 221 / 1, `process_capture` alone). boundary 97 / 0; activation 11 / 0; bdd 7, core bdd 18; `ruff_sharing` 1 / 0; `ruff_canary` 1 / 0 (112); ruff 13 / 0; doctests 33 / 0; conformance exit 0; `ruff_rule_phases` exit 0; cli replay 75 / 3 (the same three reds; ruff store 78,525,063 bytes as at 326; warm peak 114,192,384 bytes against 113,549,312); `cli_cost` 1 / 3 with 3 not run (cold 9,326 ms, CPU 54,540 ms, peak 585 MB, warm 532 ms, edit 2,617 ms); python library 138 / 0; language 43 / 0; hooks 45 / 0; `checkpoint_check` 16 / 0; `no_print` exit 0; `bench_build` exit 0; `core_allocation` exit 101 (Phase 7); `tm_source` 0 findings, 2 suppressed; `tm_validate_all` exit 0.
- **Erratum (review repairs XII): `core_bdd` 18 / 0 and `core_allocation` exit 101 are not results for `142fe40a98`.** Both test binaries took dependency units from the target directory shared across worktrees that another worktree had built with an uncommitted debug print: `core_allocation.log` holds 121 lines `DBG keep bucket …`, the literal is in both binaries and in no source of any worktree, and cargo judged the units fresh by mtime. Both are rerun at 328 on the tree's own target (method.md, "Build targets").
- **Whole-tree TM:** 18 findings, 6 suppressed, all `no-domain-concept-cycles` in `crates/execution_domain` (the ring row), as at 326. File-level ownership cycles 0 over 4,873 edge occurrences (4,864 at 326).
- **Acceptance blocks** (each at its frozen head in its own worktree, exit 0; on the merged tree without the worktree-state and branch-head lines, exit 0): review repairs XI (`run.py --check` exit 0, the record present, crates.md no longer says "all 13 runtime routes", `tm` over `docs/contracts` 0, no ignore added); perf iteration 3 (`tm` over its changed `.rs` files 0, `perf-p3.md` present, the README row present, no ignore added).
- **Django receipt (full Django 413 cold, 2,875 files x 413 rules, fresh store, under the build lock, default threads):** the release `ruff` bench worker built from `142fe40a98` after touching the tree's sources (sha256 `c9d31c91ff56d78acead49a9bbed2f07cbfae201b5e95054cebe36957644ab42`). The request was staged by `verify_profiles.py --stage-request <the pinned 323 cold request> --source 142fe40a98 --output-dir <fresh directory>` (the worker copied into that directory): staged digest `04f6c9d81a6c4a6f1c5c26395b68b33fa0b05eee81ae19ad997ab767e1f11ea1` (1,088 files; the digest of 326: rules, fragments and `benchmark.toml` did not change), request sha256 `ce5f575eb48c5df597a2c13ed6c1ace48b1b097d0006161e307b5eeececf46d5` (326's differed in the cache path only).

  | Threads | Exit | unknown_or_unread | Diagnostics | Skips | Wall s | CPU s (user + sys) | CPU / wall | Peak RSS | Load at start |
  |---|---|---|---|---|---|---|---|---|---|
  | default (10 cores) | 0 | 0 | 8,804 | 395 | 322.7 | 2,564.4 (2,503.3 + 61.1) | 7.95 | 6.79 GB (6,788,032 KB) | 1.37 |

  Coverage: selected 1,187,375, evaluated 1,186,980, skipped 395 (syntax-rejected); store 1,778,454,983 bytes, as at 326. The output bytes are byte-identical to the 326 receipt (sha256 `53b163f4bda83afa...`, 992,711 bytes). Against 326 at default threads (502.3 s, 4,062.8 s CPU, 7.00 GB): wall -36%, CPU -37%, peak -0.21 GB (perf iteration 3's fewer polls and shared text steps). The one-thread run was not asked for at 327.
- **Perf iteration 3's two lifecycle assertions** (`runtime/service/lifecycle/test.rs`), read at integration: frames now leave a region's slot when it publishes or fails, so (a) `nested_demand_preserves_both_frames_and_exact_semantic_failure` no longer reads each failed root's frame after the run (100 and 101) and asserts both slots hold none; the probe's in-call assertion stays, that the nested failure leaves the outer frame at its own value (100); the inner root's frame value is no longer asserted anywhere; (b) `downstream_error_keeps_publication_and_retry_does_not_repeat_semantics` asserts the published root holds no frames instead of one frame row; the retry's no-repeat assertions are unchanged. Named for the Rust review: (a) loses the inner-frame value check.
- **Landing 1 follow-ups and the 326 Rust gate notes:** unchanged; they stay with their row.
- **Principal Rust review:** PASS on all seven items, first pass (diff `f3a9a7e95c..dc9af4d4be`, `evidence/checkpoint/rust-review.json`); on the lifecycle test (a) it finds the inner frame separation still proven by the probe (the outer frame stays 100 after the nested call). Its two advisories (not failures) go to ryo-artq: the cost fixture should also require `staged.json`'s `request_sha256` to equal the sha256 of `request.json` (and staging should state or make the worker-beside-receipt placement); `evidence/inventory/items/execution.md:7` still lists the old `pub(super) enum Submission`, to regenerate.

**Cycle metrics:**

| Metric | Checkpoint 327 |
|---|---|
| Builders | 2 (review repairs XI, perf iteration 3) |
| BLOCKED questions | 0 |
| Time dispatch to FROZEN (first and last branch commit) | review repairs XI 10:10 to 10:13, perf iteration 3 09:21 to 10:43 (2026-11-08 UTC) |
| Tokens per builder | not supplied to the integrator |
| Compile passes | 1 check pass (errors 0) and the validation's test build |
| Rows ticked by machine | 0 (`run.py --check` un-ticks no row) |
| Lines under `crates/ryo` | +222 / -85 (Rust +151 / -70) |
| Full Django 413 cold | exit 0, unknown 0; default threads 322.7 s, 6.79 GB (502.3 s, 7.00 GB at 326), output bytes identical |
| Contract review | pending |
| Rust gate review | PASS on all seven items, first pass (diff `f3a9a7e95c..dc9af4d4be`) |

## Review repairs XI

Branch `claude/w9-review-repairs-xi` from checkpoint 326 `f3a9a7e95c`, for the contract review of 326 (NOTE on all seven phases). Checked: `run.py --check`, `run_test.py` and the profile guards `--self-test`; no Rust test was run, `cargo check -p idiomatic-sh-ryo --tests` compiled the cost rows.

- **R1, staging is enforced (`8bc1eb4e40`):** `verify_profiles.py --self-test` now runs `staged_request_self_test`, which stages from `HEAD` and checks the receipt digest against the staged files, that every rule-side field comes from the staged `benchmark.toml` and the corpus side from the pinned request, and that staging over a directory is refused. The `cli_cost` Django and one-file rows read `request.json` of a staged directory (the receipt's) and are not available (`RequestNotStaged`) without a `staged.json` that records a staged digest; they no longer cut a request from the pinned cold request.
- **R2, owners (`6aa9a47c3c`):** `run.py` fails when a red result (a `failed` count above 0 or a non-zero exit) has no owner in `OWNERS`, and strips the owned-by clause from a green one. `core_allocation` (exit 101) is owned by the perf iteration 2 row; `ruff_rule_phases` (green) no longer carries a clause.
- **R3, stale text:** README:9 and the Phase 1 row cite validation326; row "Prove actual413 sharing" cites validation326 (`ruff_sharing` and the five `walk_coverage` route and head controls pass in `tests.json`); the rows "Consolidate legal fusion", "Remove redundant intermediates", "Unify cold/warm/edit reuse" and "Prove file/project/configuration invalidation" are marked unverified at validation326; crates.md:12 says 9 kinds; the perf iteration 2 row records `relation_partitions` as a private index added.
- **Standing from 326:** rows 237 and 240 (older validations), walk B, the fixtures-gated second `derived()` composition, `RAYON_NUM_THREADS` as an environment setting, the Rust gate whitespace and `ok_or` notes (ryo-artq), the review README branch table, the 320 to 321 deletions, and README:267 "8 flagged kinds".

## Checkpoint 328 integration record

Checkpoint 328 merges four branches into checkpoint 327 (`e49dc1dfc5`), each as a merge commit, in this order: landing 2, the execution runtime crate (`claude/l2-runtime-crate` `97b22c1de3`, merge `e9b643b228`), the Phase 1 tail (`claude/w9-phase1-tail` `ef46128f82`, cut from 326 `f3a9a7e95c`, merge `57be7fb114`), review repairs XII (`claude/w10-review-repairs-xii` `6febc1a4a3`, merge `511c778990`) and, after validation, the heap profile feature (`claude/perf-heap-profile` `9f9b98a663`, merge `6fc53620b9`). Five integration fixes, below. validation328 ran on `4a982fa0a5` (raw logs in `ryo-contract-validation328-20261108`), the first validation on the tree's own target directory (`idiomatic-target/<worktree>`, review repairs XII).

- **Joins:** all four merge without conflict.
- **Integration fixes:** (1) `b24bfeda1b`: the python sharing fixture's `include_str!` of `route_meanings.txt` named the pre-move path (the one error of check pass 1). (2) `54839488ad`: the validation set covers the runtime crate: its library tests, doctests and print lint, and `tm_source` and `ownership_edges` on its moved paths. (3) `923a577b46` and `4a982fa0a5`: landing 2 left the invocation observation (`InvocationResources::observe`, its three message constants and the lease's registered-call count) behind `cfg(test)` in the runtime crate. Execution's library tests build the runtime without `cfg(test)`, so the counters stayed 0: the closure row `empty_actual_owner_keeps_its_output_and_workspace_empty` was red (`closure_partition_rows` 0 against 1, in the aborted first run), and the `closure_partitions` assertions (`== 0`, `<= 3`) passed on zeros. They are now `cfg(any(test, feature = "fixtures"))`, as the slot side already was; the observation reads the acquired partitions in place, and the test-only `AcquiredPartitions::iter` it alone used is gone (opening it tripped `fixtures-only-in-tests`). (4) `47248f1bf3`: XII's `{worktree}` expansion put the expanded target directory into the committed `identity.json` and `validation.json`, a machine path, and a worktree named for its checkpoint reads as tracker text there (44 `no-tracker-text-shipped` findings); `run.py` now runs the expanded command and records the command as `commands.json` lists it. (5) In the record commit: the heap profile paragraph of method.md named the build lock by its machine path (`no-machine-paths-shipped`); it names the build lock.
- **Named for the reviewers:** the Phase 1 tail changed the removed-file assertion for the `Always` rule: under `Always` a rejected file is answered, so the cold premise is "the rejected file's rows are among the cold rows" (cold rows differ from the oracle's), not an unknown; `AcceptedSyntax` keeps "skipped"; the warm-against-oracle assertions are unchanged. The store format marker changed (each frontier proof's rule digest is kept once; the trailer gains `proof-digests`): an older store is a cache miss and is rebuilt, with no compatibility code. The 285 reference-discrepancy settings digests were rebound (the settings text lost two retired options at their defaults). The observation fix means the `fixtures` build of the ruff bench worker (the self dev-dependency enables `fixtures`) now runs the invocation observation per lease, which the 327 worker did not.
- **Owed by the 327 erratum:** `core_bdd` 18 scenarios / 0 failed and `core_allocation` exit 101 (a file allocates 1,698 to 1,700 blocks beyond the parser library; Phase 7) on the tree's own target.
- **Landing 2's open risks:** the runtime's doctests run for the first time and pass 4 / 0, the two `compile_fail,E0599` probes (`FileExecution`, `ProjectRootExecution`) among them. The evidence inventory and `visualization/evidence.json` still name the pre-move paths; review repairs XII regenerated the inventory items for 327's tree, and the moved paths are not regenerated here. Landing 2's deferred scope (one engine call per project phase, readiness from store proofs, `prepare` taking a request) stays with its follow-up bead.
- **Heap profile (merged after validation):** a `heap-profile` feature on the Python crate (`tikv-jemallocator/profiling`, off by default) and the "Heap profile." paragraph in method.md. `--all-features` builds take it, so the python targets ran again on `6fc53620b9` (jemalloc rebuilt with `--enable-prof`): `python_lib` 138 / 0, `ruff_sharing` 1 / 0, `ruff_canary` 1 / 0, `ruff` 13 / 0, conformance exit 0 (`rerun-<target>-6fc53620b9.log`). The receipt worker is built without it.
- **Acceptance blocks** (each at its frozen head in its own worktree under `bash -e`, exit 0): landing 2, the Phase 1 tail, review repairs XII, heap profile. Under GNU grep the blocks' ignore-grep line is loose (its unescaped ignore-attribute pattern is a bracket class): on the Phase 1 tail it matches Python comments in the rebound `cases.toml` text, and with the unescaped allow pattern it is a regex error that passes vacuously; the escaped form (`#\[ignore\]|#\[allow\(`) finds 0 added on every branch and on this tree.

| Pass | Command | Errors reported | Errors fixed |
|---|---|---|---|
| 1 | workspace check (all features, all targets), landing 2 and the Phase 1 tail merged | 1: the sharing fixture's `include_str!` of a moved file | 1 |
| 2 | the same | 0 | 0 |
| 3 | the same, review repairs XII merged | 0 | 0 |
| 4 | the same, the observation opened under `fixtures` | 0 | 0 |
| 5 | the same, the observation reading the list in place | 1: `AcquiredPartitions::iter` never used (`-D dead-code`) | 1 |
| 6 | the same | 0 | 0 |

Each pass is the workspace check with `--keep-going` and JSON diagnostics into this tree's check directory, under the build lock; the logs are in `check-passes/` of the validation directory. Warnings: 4,640 at 327, 4,672 at pass 6: `dead_code_pub_in_binary` +36 (the runtime's library test target, the price of the crate split landing 2 names) and `unused_qualifications` -4 (the observation's `super::` paths); no warning falls on a line this checkpoint added that 327 did not carry at the old path.

Production and test lines (`src` only; test = `*_test.rs`, `test.rs`, `fixtures.rs`, `fixtures/`, `*_test/`):

| Crate | 327 production | 327 test | 328 production | 328 test |
|---|---:|---:|---:|---:|
| cli | 3,420 | 2,292 | 3,421 | 2,292 |
| crates/benchmark | 23,279 | 25,815 | 23,283 | 25,815 |
| crates/core | 19,731 | 24,211 | 19,757 | 24,220 |
| crates/execution | 18,944 | 32,128 | 2,393 | 10,625 |
| crates/execution_runtime | — | — | 16,662 | 21,477 |
| crates/operators | 5,285 | 5,086 | 5,295 | 5,086 |
| crates/sdk | 6,032 | 3,979 | 6,034 | 3,979 |

Every other crate is unchanged. Under `crates/ryo` (renames detected) the checkpoint adds 2,767 lines and deletes 2,376 over 666 files (Rust +1,248 / -1,028 over 550 files, nearly all path rewrites).

- **validation328 (rows, from `tests.json` and the raw logs):** libraries 1080 / 1 (`process_capture` only; core 121 / 0 with the removed-file row green; execution 72 and runtime 198, the 270 rows of 327's execution library); boundary 97 / 0; activation 11 / 0; bdd 7, core bdd 18; doctests all pass (runtime 4); `ruff_sharing` 1 / 0; `ruff_canary` 1 / 0 (112); ruff 13 / 0; conformance exit 0; `ruff_rule_phases` exit 0; cli replay 76 / 2 (the cache size row passes; the feed row now reports `family:FloatTextGreaterThanOrEqual`, the shared walk's per-step count; warm peak 114,536,448 bytes against 114,192,384 at 327); `cli_cost` 4 / 3 (cold 9,039 ms, peak 589 MB, warm 542 ms); python library 138 / 0; language 43 / 0; hooks 48 / 0; `checkpoint_check` 16 / 0; `no_print` exit 0; `bench_build` exit 0; `core_allocation` exit 101 (Phase 7); `tm_source` 0 findings; `tm_validate_all` exit 0. A first start of validation328 on `54839488ad` was stopped: an edit landed in the tree while it ran; its logs are kept in `aborted-54839488ad/` of the validation directory and are not results.
- **Whole-tree TM:** 18 findings, 6 suppressed, all `no-domain-concept-cycles` in `crates/execution_domain` (the ring row), as at 327. File-level ownership cycles 0 over 4,645 edge occurrences in execution, its domain, runtime and store (4,873 over three crates at 327).
- **413 inventory:** `verify_profiles.py <ruff 0.16.5> --full-inventory --worker <the receipt worker> --inventory-runner <the collector built from 6fc53620b9>`: **413 PASS / 0 FAIL**, exit 0, 376 s (collector sha256 `0d786b214a5e8840136925c43a6a35743b4e64cc569c3fbbe82cb1025c7b88a2`).
- **Django receipt (full Django 413 cold, 2,875 files x 413 rules, fresh store, under the build lock, default threads):** the `ruff` bench worker built from `6fc53620b9` into the tree's own target and copied under the same lock (sha256 `3ac82ac8b05ef12476c93481d9de7e7a4eb1cf3955b0a432f60237aab4d0f5fb`, features `fixtures` only; it carries the `proof-digests` trailer and the runtime crate's paths, no `DBG ` string). The request was staged by `verify_profiles.py --stage-request <the pinned 323 cold request> --source 6fc53620b9 --output-dir <fresh directory>`: staged digest `0132c274d68dc22252b09d0551a36617652fd7ffecaff02a246e5bf0448027df` (1,088 files; against 327 only the 42 rebound `cases.toml` files differ), request sha256 `6980cae6aa67a722c384b061670498119220dc67f671fac6ea3cb5f7f2a288aa` (327's differs in the profile and cache paths only).

  | Threads | Exit | unknown_or_unread | Diagnostics | Skips | Wall s | CPU s (user + sys) | CPU / wall | Peak RSS | Load at start |
  |---|---|---|---|---|---|---|---|---|---|
  | default (10 cores) | 0 | 0 | 8,804 | 395 | 328.3 | 2,641.2 (2,571.6 + 69.6) | 8.04 | 6.78 GB (6,781,524 KB) | 1.10 |

  Coverage: selected 1,187,375, evaluated 1,186,980, skipped 395 (syntax-rejected). The output bytes are byte-identical to the 327 receipt (sha256 `53b163f4bda83afa...`, 992,711 bytes): the store format change touches no diagnostic. Store 1,757,210,648 bytes (1,778,454,983 at 327: each proof's rule digest kept once). Against 327 (322.7 s, 2,564.4 s CPU, 6.79 GB): wall +1.7%, CPU +3.0%, peak -0.01 GB; the fixtures-gated observation per lease is in this worker and not in 327's.
- **The staged cost rows** (`RYO_PREFLIGHT_WORKER` naming the receipt worker; the staged digest and the request's sha256 recomputed on the real staged directory, XII's check, hold): `an_absent_input_is_a_typed_reason_not_a_pass` passes (its `StagedDiffers` cases also passed in validation328); N999 over Django wall 10,393 ms and CPU 10,360 ms hold (best 10,297 / 10,294), peak 1,062 MB against best 979 (red; 1,056 at 327); one file x 413 wall 3,334 ms and peak 889 MB beat best 6,871 ms / 1,139 MB by more than 5% (3,429 ms / 860 MB at 327). `ratchet.toml` is not changed here. The full-Django cold-and-warm row was not run.

- **Principal Rust review:** PASS on all seven items, first pass, no findings (diff `e49dc1dfc5..490bc0f3c2`, `evidence/checkpoint/rust-review.json`). Its five advisories (not failures): A1, perf evidence: the invocation observation is now compiled into the fixtures-on ruff bench worker behind every 328 perf number, so the +1.7% wall and +3.0% CPU against 327 cannot be split between the observation and the engine; the fix is to move the execution use-case proofs that read these counters into the runtime crate as `cfg(test)` tests and return the gate to `cfg(test)`, or until then to record each receipt's worker features and A/B one Django receipt with the observation compiled out. It is step 0 of perf iteration 4. A2, surface: `program::admission` and `file_dag::service` are `pub` modules with no consumer outside the runtime; narrow to `pub(crate) mod`. A3, store: `rest/schemas.rs:160` maps a digest-table index overflow to `u32::MAX`, which past 2^32 entries would alias a real entry; return an encode error. A4, fixture: `workload.rs:120` `is_dir()` follows symlinks where the Python staged-hash walk may not; state or match it. A5, docs: the evidence inventory and `visualization/evidence.json` still name the pre-move `file_dag` paths; regenerate. A2 to A5 go to the Rust gate notes row.

**Cycle metrics:**

| Metric | Checkpoint 328 |
|---|---|
| Builders | 4 (landing 2, Phase 1 tail, review repairs XII, heap profile) |
| BLOCKED questions | 0 |
| Time dispatch to FROZEN (first and last branch commit) | landing 2 12:05 to 12:26, Phase 1 tail 10:52 to 11:55, review repairs XII 12:23 to 12:34, heap profile 12:23 to 12:49 (2026-11-08 UTC) |
| Tokens per builder | not supplied to the integrator |
| Compile passes | 6 check passes (errors 1, 0, 0, 0, 1, 0) and the validation's test build |
| Rows ticked by machine | 0 (`run.py --check` un-ticks no row) |
| Lines under `crates/ryo` | +2,767 / -2,376 (Rust +1,248 / -1,028) |
| Full Django 413 cold | exit 0, unknown 0; default threads 328.3 s, 6.78 GB (322.7 s, 6.79 GB at 327), output bytes identical |
| 413 inventory | 413 PASS / 0 FAIL |
| Contract review | pending |
| Rust gate review | PASS on all seven items, first pass (diff `e49dc1dfc5..490bc0f3c2`), no findings, five advisories |

## Checkpoint 329 integration record

Checkpoint 329 merges three branches into checkpoint 328 (`90a0a472b2`), each as a merge commit, in this order: engine fixes after landing 2 (`claude/w11-engine-fixes` `5cc10afcec`, merge `9891c2fd1d`), the runtime follow-up (`claude/w11-runtime-followup` `828262ad34`, merge `a9fa5ed4f6`) and review repairs XIII (`claude/w11-review-repairs-xiii` `b4a3823d14`, merge `74a799c116`). validation329 ran on `74a799c116` (raw logs in `ryo-contract-validation329-20261108`) on the tree's own target.

- **Checks:** the workspace check (all features, all targets, `--keep-going`) ran once, 0 errors, 4,646 warnings (4,672 at 328 pass 6); one check pass against 6 at 328.
- **Replay feed row:** `behavior::ruff_rules_feed_every_catalog_feature_a_thousand_rows` is green. The engine fixes ruling (every comparison of a step is credited with the step's rows) dropped the `FloatTextGreaterThanOrEqual` entry of `unfed.toml`, and the stale entry of the hand-kept results is removed. cli replay 77 / 1: the warm peak row stays red (`peak::a_warm_unchanged_run_over_ruff_peaks_within_its_bound`).
- **Known reds, as at 328:** `process_capture` succeeds (libraries 1 failed) and `core_allocation` (exit 101, Phase 7); the `cli_cost` ruff ratchet rows (3) and the warm peak row.
- **Django receipt (full Django 413 cold, 2,875 files x 413 rules, fresh store, under the build lock, default threads):** the `ruff` bench worker built from `74a799c116` into the tree's own target (`idiomatic-target/<worktree>`) (`cargo bench --no-run -p idiomatic-sh-ryo-python --bench ruff`) and copied under the same lock, sha256 `70587a223150979f79753987cf6643a5514fd430480b6f38ae04529798edc691`, no `DBG ` string (`strings | grep -c` 0). The request was staged from `74a799c116` (staged digest `0132c274d68dc22252b09d0551a36617652fd7ffecaff02a246e5bf0448027df`, 1,088 files, the same as 328; re-staged into a fresh directory before the run: same digest, the request differs from the re-stage only in the profile and cache paths), request sha256 `406d0b15b185db249ae7651a374b1d0cb1f6c8f4f7819c32b1e708522bd5976d`.

  | Run | Exit | unknown_or_unread | Diagnostics | Skips | Wall s | CPU s (user + sys) | CPU / wall | Peak RSS | Load at start |
  |---|---|---|---|---|---|---|---|---|---|
  | 1 | 0 | 0 | 8,804 | 395 | 337.4 | 2,696.7 (2,627.9 + 68.8) | 8.00 | 6.71 GB (6,711,972 KB) | 7.26 |
  | 2 (rerun, load above 3 at run 1) | 0 | 0 | 8,804 | 395 | 327.5 | 2,647.7 (2,580.7 + 67.0) | 8.08 | 6.78 GB (6,780,164 KB) | 6.16 |

  Both outputs are byte-identical to 328's (sha256 `53b163f4bda83afaac6c5b7ff7c11bba65532c1de6f664bc75fabc52cbb547b4`, 992,711 bytes); coverage selected 1,187,375, evaluated 1,186,980, skipped 395. Store 1,757,210,648 bytes (unchanged from 328). The other trees' builds kept the machine's load at 6 to 7 during both runs, so the walls carry that noise: run 2 against 328 (328.3 s, 2,641.2 s CPU, 6.78 GB): wall -0.2%, CPU +0.2%, peak 0.00 GB.

- **Rust gate:** PASS on the third pass (first pass FAIL 2,4; second pass FAIL 4).
- **Gate repairs after validation329** (not run by it; the tree's next validation runs them): `779f4a960f` (production) and `d554903903` (tests), for the Rust gate's passes 1 and 2.
  - `779f4a960f`: the prepared program and the file session hold their members in the crate-private `ProgramParts` and `SessionParts`, so `private_interfaces` refuses a widened `parts`; 12 of the 15 `compile_fail` probes review repairs XIII restored are deleted (3 remain); the second `library-surface` entry (over `crates/execution/**`, XIII) is dropped. The phase call becomes `ProjectRootExecution::evaluate_occurrences` over `PhaseOccurrences`, and `ExecutionRequest::ordinary_phase` and `OrdinaryRules` are gone. Deleted from core: `PreparedProject`, `ReadyProject`, `CrossFile::evaluate_batch`, the preparation module (`Preparation`, `SealedBatch`, `BatchPreparationError` and `CrossFileError::Preparation`), its cases and fixtures, and their placements. On error paths only, an observation occurrence with no first bucket now answers `MISSING_FRONTIER_BUCKET`, and its scope and inapplicable refusals are named for its rule.
  - `d554903903`: core's multi-occurrence cases (the budget case and the three root-set cases) go through `evaluate_occurrences`; the budget case passes its `within` check as the phase's guard; `evaluate_prepared` is deleted.

**Cycle metrics:**

| Metric | Checkpoint 329 |
|---|---|
| Builders | 3 (engine fixes, runtime follow-up, review repairs XIII) |
| Compile passes | 1 check pass (errors 0) and the validation's test build |
| Full Django 413 cold | exit 0, unknown 0; default threads 327.5 s, 6.78 GB (328.3 s, 6.78 GB at 328), output bytes identical |
| Contract review | pending |
| Rust gate review | PASS on the third pass, at `d554903903` ([rust-review.json](evidence/checkpoint/rust-review.json)) |

## Engine fixes after landing 2

Branch `claude/w11-engine-fixes` from checkpoint 328 `90a0a472b2`. Checked: `cargo check -p idiomatic-sh-ryo-execution-domain -p idiomatic-sh-ryo-execution-runtime --all-targets` compiles; `tm` over the changed files at 0 findings; no test was run (the integrator runs them). Counters, reuse scopes and one replay fixture entry: no output byte changes.

- **The `Declined` evaluation is counted by the queue:** a region that declines the shared walk is still evaluated inside its preparation poll, but the runtime no longer adds to the evaluation count by hand. The poll answers that it evaluated, and `ReadyQueue::evaluated_in_preparation` counts it as one of its own: once per poll, only for a `Prepare` action. The queue is again the only writer of `ReadinessCounts`. An evaluation is counted per attempt on both paths, so an inline evaluation that suspends and resumes by dispatch counts as the same region evaluated only by dispatch. Controls: `ready_queue/value_test.rs`, "an evaluation in preparation is counted once by the queue" (the suspended inline script against the dispatched one) and "... is recorded only for a preparation poll"; `runtime/service/lifecycle/test.rs` `a_region_evaluated_in_its_preparation_is_counted_once_as_a_dispatched_one_is` (the `Declined` and `Submitted` paths, one preparation and one evaluation each).
- **Request-level reuse is an exhaustive choice:** `operator/contract/service.rs` `reuse_scope` matches every `Effects` variant, so a new effect does not compile until it says whether it is request-shared. `RequestSource` stays rule-scoped. The negative control is `request_graph_test.rs` `equal_request_source_steps_of_different_rules_keep_their_own_producers`: two rules' equal `Hosted` steps, which bind nothing, keep two producers. The stale `file_program` comment ("identical pure terminals") now names the reuse scope.
- **The replay feed keeps the per-step model (ruling):** the shared walk credits a select with the rows it offers the step's class. `lossy-float-literals` plans as one fused `Select` over `syntax-tree` (`text != ""`, `kind = "float-number"`, `float_text_gte`; `fuse_selects` folds the chain), and the ordinary route feeds that step every row too, so 9,763,518 is the step's feed. The ruling keeps the per-step feature model, the simpler one: every comparison of a step is credited with the step's rows. The feature model and the engine are unchanged. `cli/tests/fixtures/unfed.toml` states that rule in its header and drops `family:FloatTextGreaterThanOrEqual`, whose reason ("0 rows reach the comparison") was a per-comparison claim. No other entry was stale for that reason: the other family entries (`EndsWith`, `NotGlobAny`, `StartsWith`, `NaturalLessThan`) are used by no recorded rule, so they are unfed under either model.
- **Not repaired here, undetermined acceptance answering Unknown (open until after checkpoint 329):** the files a rule was refused on (`SyntaxUndetermined`) are known only to core's file analysis. The frontier proof needs them in the rule's scope to answer a refused frontier instead of `PROOF_MISMATCH`. That runs through core `cross_file` and the project-phase scope and prepare calls, which the runtime follow-up builder owns on this round.

## Review repairs XII

Branch `claude/w10-review-repairs-xii` from checkpoint 327 `e49dc1dfc5`, for the contract review of 327 (Phase 7 BLOCKING: two validation327 binaries carried code from no tree). Checked: the hook tests (48/0), `run.py --check`, `run_test.py` (16/0) and `tm` over `docs/contracts` and the changed cost files at 0 findings, 0 suppressed; no Rust test was run, `cargo check -p idiomatic-sh-ryo --tests` compiled the cost rows.

- **R1, one cargo target per worktree (`255133cead`, `8ba71711d8`):** `.claude/hooks/guard_shell.py` accepts `--target-dir` only as `idiomatic-target/<this worktree's name>` in the workspace mount (the check-only target rules unchanged) and refuses another tree's name, the bare root, a deeper path and the shared target, with no exception for the retired shared target. `commands.json` names `idiomatic-target/{worktree}` and `run.py` expands it to the tree's name, so every validation command builds only this tree's units; the source touch the shared target needed is gone. method.md "Build targets" and AGENTS.md state it, and that deleting a worktree deletes its target directory. The live hooks run this head.
- **R2, the 327 erratum (`8ba71711d8`):** `core_bdd` 18 / 0 and `core_allocation` exit 101 are not results for `142fe40a98` (the 327 record and validation.json say so); both run again at 328 on the tree's own target.
- **R3, `core_allocation` owner (`8ba71711d8`):** red at every validation from 318, it is owned by "Measure full-Django cold/warm/edit and Rust regressions; meet final objective.", which decides its block limit, not by perf iteration 2.
- **R4, the staged request is checked (`16f97e9d59`):** `cli_cost`'s `inputs_at` recomputes the staged digest of `profile/` and the sha256 of `request.json` and refuses a `staged.json` that records either otherwise (`NotAvailable::StagedDiffers`); `an_absent_input_is_a_typed_reason_not_a_pass` covers both mismatches with the digest `--stage-request` gives its files. No flag or variable added.
- **R5, stale text (`0fa87fec2f`):** README:20 cites the 327 receipt; the per-kind row cites the inventory regenerated on the 327 source (10 kinds; `registries.py` now reads the kinds and routes in `execution_domain` and `execution_store`); the review README branch table runs to 327; rows rebound to validation327 or marked unverified; `items/execution.md` lists `pub(crate) enum Submission` under `program/runtime`; the benchmark reruns, the three staged `cli_cost` runs and the whole-tree TM are in the validation327 raw directory, and `run.py`'s rerun check now also reads "reruns".
- **Not here:** the engine items of review 327 (the `Declined` double count and its test, the exhaustive `Effects` match with a `RequestSource` negative row, the stale `file_program` comment) are program code, after landing 2.

## Review repairs XIII

Branch `claude/w11-review-repairs-xiii` from checkpoint 328 `90a0a472b2`, for the contract review of 328 (Phase 2 and Phase 4 BLOCKING). Checked: a package-scoped check of the runtime and core, all targets (the runtime's warnings unchanged, path-normalized), `run.py --check`, and `tm` over the changed files at 0 findings; no test or doctest was run, so the restored probes run at the next validation.

- **R1, the privacy probes (`prepared_execution/types.rs`):** the six probes landing 2 deleted come back as runtime doctests on the runtime paths, with exact codes: E0603 on `file_dag::execution::CommonOperatorEngine`, E0616 on `PreparedExecutionProgram.inner` and `FileSession.inner`, E0451 on a literal of either, E0624 on `RequestExecution::open_project`. The review's falsifier was `pub kernels` (0 findings), which none of the six catches (the literal still fails on the other fields), so each other private field gets its own E0616 probe: `kernels`, `derived`, `digest`, and `program`, `file`, `elements`, `read`. The runtime holds 15 `compile_fail` doctests (2 at 328). The four E0432 probes of removed names are not restored: they named `ryo_execution` paths no item reaches. The doc text no longer says rustc refuses a widening on every build of its callers.
- **R1, the library surface (`tastemaker.toml`):** a second `library-surface` entry over `crates/execution/**/*.rs` lists the runtime paths execution may name: the three outbound ports, the request protocol's modules (`prepared_execution`, `request_execution`, `file_session`, `project_phase`, `observation_stage`, `observed_publication`, `rule_outcome`, `preparation`, `frontiers`, `observations`) and the fixtures-gated `acceptance`, `fixtures` and `testing`; the surface is what execution named at 328 (516 paths; 0 findings with the entry). Proof on a throwaway copy of the tree: execution naming `program::admission::ApplicabilityProof` and `file_dag::service::prepare` gave two `library-surface` findings, reverted after. A widened field writes no path, so `pub kernels` still gives 0 `tm` findings; its E0616 probe is what fails.
- **R1, Rust gate A2:** not applied. `program::admission` crate-private does not compile (`unnameable_types`: `RootOutcome::Skipped` carries `ApplicabilityProof`, which core matches), and `file_dag::service` crate-private is an `application-module#template` finding (the template requires `pub mod service`; every item in it is crate-private). Both are now outside execution's surface, and core, the SDK and the command line were already held to the ports.
- **R2, the crate map:** `crates.md` rows 14-20 and the summary name the four engine crates as they are: execution composes the request over the runtime's two entries; the runtime owns scheduling, admission and lifetime, the store the derived views, publications and leases; the store row names the runtime as a dependant; the runtime row is compiled and validated (198 / 0, 16,662 production lines) and states why `admission` and `service` are `pub`; the core, SDK and command line rows name the runtime's ports edge, the SDK and command line rows execution too. The step 3 row carries the landing 2 result, the `core_allocation` count is validation328's, and `execution_store` takes its place in the `crate-layers` order between the runtime and the domain.
- **R3, the removed file (`lint/fixtures/skips.rs`):** under `Always`, the cold run must hold a row naming `b.rs` (the fake language names each function by its file's path) and the oracle none, in place of `cold.rows != oracle.rows`.
- **R4, the inventory:** regenerated on this tree with `ryo 0.1.1`; `render.py` maps `ryo_execution_domain`, `ryo_execution_store` and `ryo_execution_runtime` to their crates and `registries.py` reads the runtime's kinds and routes, so the runtime has its concepts, imports and items pages and no runtime path is filed under execution. Still 10 flagged kinds. `visualization/evidence.json` is not regenerated here.

## Checkpoint 330 integration record

Checkpoint 330 merges perf iteration 4 (`claude/p4-memory`, merge `8d9239677d`) and perf iteration 5 (`claude/p5-plan-rewrites`, merge `2a0bec2965`) into checkpoint 329 (`74a799c116` and its Rust gate commits), plus the engine edits `f472801258` (unknown rule index refused, project-batch-owner 0.1.1, the two-reviewer charter, the heap profile interval and peak loop) and `4df6abaca2` (project-batch-owner 0.1.0 deleted, the 329 contract review evidence). validation330 ran on `4df6abaca2` (raw logs in `ryo-contract-validation330-20261108`) on the tree's own target.

- **Checks:** the workspace check (all features, all targets) ran once, 0 errors, 4,630 warnings (4,646 at 329); one check pass.
- **Validation:** libraries 1085 PASS / 1 failed (execution 73, core 119, runtime 200; 1080/1 at 329), `core_bdd` 18 scenarios / 0 failed, `ruff` 13 / 0, execution boundary 97 / 0, whole-tree TM 18 findings with 6 suppressed, file-level ownership cycles 0. Known reds, as at 329: `process_capture` succeeds, `core_allocation` (exit 101), `cli_tests` warm peak, and the three `cli_cost` ruff ratchet rows.
- **Rust gate 330 repairs:** `UnknownRuleIndex` maps to core `LintError::Execution` (no fabricated `RuleId`), `FrameOffset` carries a doctest, validation.json rebound to the raw logs; error path and docs only, no Django rerun (core lib 119/0, values doctests 4/0, `cargo check --all-targets` on core and values clean).
- **Django receipt (full Django 413 cold, 2,875 files x 413 rules, fresh store, under the measure and build locks, default threads):** the `ruff` bench worker built from `4df6abaca2` into the tree's own target (`cargo bench --no-run -p idiomatic-sh-ryo-python --bench ruff`) and copied under the same lock to a new directory as `worker-4df6abaca2`, sha256 `f1ba077013da3df5da94439a2468ff2379f7583ec87bc566a5549b365e6a59e5`, no `DBG ` string (`strings | grep -c` 0); `build.json` beside it ends with the provenance line (commit, sha256, build time 20:36:32Z after the commit time 19:24:10Z). The request was staged by `verify_profiles.py --stage-request <the pinned 329 cold request> --source 4df6abaca2` (staged digest `0132c274d68dc22252b09d0551a36617652fd7ffecaff02a246e5bf0448027df`, 1,088 files, the same as 328 and 329), request sha256 `314aa80c2475fd30d8330e015a4594ff4d12e07f73f84dd576b1558bbf6591e8`.

  | Run | Exit | Output bytes | Wall s | CPU s (user + sys) | Peak RSS | Load at start |
  |---|---|---|---|---|---|---|
  | 1 | 0 | 992,711, sha256 `53b163f4bda83afaac6c5b7ff7c11bba65532c1de6f664bc75fabc52cbb547b4` (identical to 329) | 428.0 | 2,752.2 (2,702.4 + 49.8) | 3.33 GB (3,334,596 KB) | 2.56 |

  Peak RSS 6.78 GB at 329 to 3.33 GB (perf 5 measured 3.63 GB). The wall is not a speed claim: other trees' work raised the load to about 12 during the run (12.54 at its end), so 428.0 s against 327.5 s at 329 carries that noise; the CPU rose 4% (2,752 against 2,648 s). The store after the run was 62,022,326 bytes against 1,757,210,648 at 329, a difference not investigated here.

**Cycle metrics:**

| Metric | Checkpoint 330 |
|---|---|
| Builders | perf 4, perf 5 (merged), plus the engine edits |
| Compile passes | 1 check pass (errors 0) and the validation's test build |
| Full Django 413 cold | exit 0, output identical; default threads 428.0 s (loaded machine), 3.33 GB peak (6.78 GB at 329) |
| Contract review | 329's evidence merged; 330 pending |
| Rust gate review | PASS on the second pass (first FAIL item 1: F1.1, F1.2), at `055b130c25` ([rust-review.json](evidence/checkpoint/rust-review.json)) |
