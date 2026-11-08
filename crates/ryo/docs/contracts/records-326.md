# Integration records from checkpoint 326

The integration records from checkpoint 326 and the branch records they merged, newest checkpoint last; [records.md](records.md) holds checkpoints 319 to 325 (at its `file-length` limit), and the current checkpoint's record is in [the matrix](README.md).

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
