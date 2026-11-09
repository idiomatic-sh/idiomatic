# Contract review: landed checkpoint 332

- Source: `dcea2ee7df7c02711f42c332cbc403ff7b84a034` (claude/ryo-consolidation)
- Tree: `052d3bcc93`
- Base: `b1e81f7242`
- Validated source of the receipts: `7fd2dbf1f5`
- Criteria: `3d1e6d91a4:.agents/agents/contract-review.md`, four criteria, Change isolation included
- Reviewer: contract-review, single reviewer
- Date: 2026-11-09
- Verdict: **BLOCKING**

| Criterion | Verdict |
|---|---|
| Enforced boundaries | PASS (bounded) |
| Clear ownership and flow | PASS |
| Proof | **FAIL** |
| Change isolation | **FAIL**: (a) FAIL, (b) FAIL, (c) FAIL |

HEAD was dcea2ee7df before and after the review, with 6 untracked files both times (reviewer drafts under `evidence/branches/`). I made no edits, builds or test runs. I ran tm and `run.py --check` on a git archive of the source. The archive's content is equal to the landed tree; it leaves 243 gitignored `results/` files out of the archive's index only. After `run.py --check` the archive was unchanged.

## Receipts

| Receipt | Command | Exit | Raw result | Verdict |
|---|---|---|---|---|
| tm | `tm --root crates/ryo check --format json` (tm 0.1.2 release, sha256 `f335d4ab`, built from `781653e221`, which predates the tastemaker refactor `beb55b206a`) | 0 (208 s) | 0 findings, 13 suppressed, all `idiomatic.arch/file-length` in-source ignores with reasons (7 accepted by the owner decision in `7d7888247c`) | PASS (manual judgment: the agent file says "0 suppressed") |
| run.py | `python3 -I .../checkpoint/run.py --check` | 0 | 17 "newly scoped ... (0 findings)" lines | PASS |
| no_print | `/tmp/ryo-contract-validation332/no_print.json` | 0 | 0 lines starting `error` | PASS |
| file-level cycles | Tarjan SCC recount over validation332 `ownership_edges.log` (source `7fd2dbf1f5`) | n/a | 0 cycles, 4,797 occurrences, 756 files | PASS (the record is stale, see P3) |

## Recounts against the record (validation332 raw logs)

Every recount matches the record:

- libraries: 1123 / 3 in libraries.log. Adding the benchmark rerun (229 / 1 in gatefix_benchmark_lib.log) gives the recorded 1125 / 1.
- boundary 99 / 0, activation 11, bdd 9 scenarios, core_bdd 18.
- ruff 12 / 0, cli replay 96 / 3, cli_cost 4 / 3.
- python 140 / 0, language 43 / 0, harness_hooks 136, checkpoint_check 16.
- tm validate 241 / 0 and 224 / 0.
- Ruff parity: 412 identical plus FURB122 = 413.

## Blockers

1. **P1. Proof: the landed production code has no full-suite run.** All suite receipts are at `7fd2dbf1f5`. Production code changed after that source:
   - `c2ba5669a6` and `bec665436a`: core handing, recording, streaming, outcome and the lint service.
   - `10c561ef6a`: execution_domain `matching/types.rs` and execution_store `derived/owner/setup.rs`.

   The reruns after those commits cover only core lib 120 / 0, core groups 11, lifecycle 2, samples 2, doctests and the benchmark library. Libraries, boundary, cli replay, cli_cost (its warm and edit rows read what t611 changed), sdk, python and ruff were not rerun, and no rerun log records its sha. **Required:** rerun those suites on `781653e221` or `dcea2ee7df` (the engine code is identical: the diff under crates/ryo/{crates,languages,cli} is empty), with the sha in each raw json and log.
2. **P2. A matrix row is stale.** `crates/ryo/docs/contracts/README.md:52` (row 1) reads "Python ruff target is 13/0 at validation328". validation332 records 12 / 0, because leak-repairs-b moved engine_test's assertions to their owners. **Owner:** the integrator rebinds the row.
3. **C1. Change isolation (a): tests reach behind the seams.**
   - `7062b9f535` deleted execution's nine forwards, and execution's own tests now import the domain's internal paths: `crates/execution/tests/test.rs:20,24`, `tests/boundary_fixture/mod.rs`, `tests/publication/frames/test.rs` and `use_case_test/{dispatch,errored_rule}/test.rs` name `ryo_execution_domain::domain::rule_intent::types::{EmitDemand,RuleDemand}` and `domain::finish::types::PublicationReason`.
   - `51e528fb5c` adds `execution/tests/project_phase/axis`, which names `ryo_execution_runtime::application::file_dag::program::file_session::types` and `ryo_execution_domain::domain::frontier_capture::types`.
   - Runtime tests consume domain fixtures (`self_join_recogniser::fixtures`, `operating::fixtures`).
   - Recount: test files outside execution_domain that name `ryo_execution_domain` went from 143 at b1e81f7242 to 154 at dcea2ee7df.

   **Required:** move each assertion to the crate that owns its subject, or reach the subject through execution's use cases. The recount must come back at or below 143.
4. **C2. Change isolation (b): the domain-cycle footprint.** Merge `9a130bb449` touched 95 files: 50 in execution_domain, 36 in execution_runtime and 8 in execution_store. The 44 runtime and store files only rewrite paths (186 changed lines, for example `operator::contract::views::ProgressObserver` to `operating::types::ProgressObserver`). The concept, breaking the ring, needed none of them. Consumers import `domain::<concept>::<role>` paths, so the domain's module layout is their API. **Required:** a stable cross-crate surface for execution_domain's vocabulary, plus a TM rule that refuses deep-path imports from other crates (route to the tm-specialist). Proof: a planted move inside the domain edits 0 files outside it.
5. **C3. Change isolation (c): the census regresses.** Graded against the scratchpad `census-trend.md`, because no census is committed at 331 or 332; the baseline commits with 333. The census's 332 column (`781653e221`) has engine code equal to dcea2ee7df. I recounted each delta below at b1e81f7242 and dcea2ee7df.

   | Row | 331 | 332 | Cause |
   |---|---|---|---|
   | Exhaustive `Operation` matches (8 or more variants) | 23 | 24 | `query_language/src/domain/refusing/service.rs` (span-semantics `dd7c0befdf`); there is no operator registry |
   | Files naming GroupCount | 42 | 43 | the same file |
   | Files naming `Operation::X` | 67 | 68 | the same file |
   | Test files naming `ryo_execution_domain` | 143 | 154 | `7062b9f535`: 5; `51e528fb5c`: 2; `e0ad1b1895` splits: 4 |
   | Engine files naming `ryo_operators::` | 25 | 26 | domain-cycle: `operating/{service,types}.rs` and `table_reach/borrowed.rs` replace `operator/contract/{primitive,service}.rs` |
   | Files naming store concrete types | 51 | 52 | `execution_store/.../derived/owner/setup.rs`, inside its own home, so neutral for isolation |
   | execution/tests files | 104 | 105 | the new axis tests |
   | Engine production lines | 123,873 | 124,960 | n/a |
   | Replace the evaluator (headline) | 180 | 179 | better |
   | Add semantic analysis | 23/13, 84/18 | 23/13, 84/18 | flat |

   **Required:** remove each rise, or record an owner-accepted reason per row. Commit the census so that 333 is graded against a committed baseline.

## Footprint per merge, (b)

| Merge | Files | Areas | Judgment |
|---|---|---|---|
| tm-import-reading (`0f2fa35a12`, `69caa24a4e`) | 12 + 7 | tastemaker, ryo provider-imports spec | inside its concept |
| t611 (`2c94c8e3d5`) | 9 | core | inside its concept |
| goal-rows | 4 | docs | n/a |
| leak-repairs-b (`643fdbf422`) | 64 | 11 areas | a multi-class repair batch that moves assertions to their owners; judged per class (manual judgment) |
| visibility (`c2a9bc8f9e`) | 9 | python 7, language 2 | justified: it reuses the engine's carrier lookup and keeps no second scan |
| bench-parity (`95319943b3`) | 127 | benchmark 50, python 68, contracts 4 | justified: one parity owner, and the Python mode is deleted |
| hooks-append-only, test-practices | 11, 17 | .claude, ci, .agents | n/a |
| domain-cycle (`9a130bb449`) | 95 | domain 50, runtime 36, store 8 | **FAIL** (C2) |
| side-doors (`310b0be96f`) | 32 | execution 21, runtime 9 | justified: it tightens the seam itself |
| span-semantics (`165c2c793f`) | 19 | query_language 8, cli replay 8, domain operation_schema 2 | justified: span meaning lives on the operation schema |
| tm-rebuild (`d5ff1b0d2e`) | 27 | 4 engine crates, tastemaker | several fixes; its new axis tests are C1 |

## Notes

- **P3.** validation.json `ownership_graph` still cites `340808449f`, 4,741 occurrences and 745 files from validation331, although validation332's log exists. `scope` reads "checkpoint 330". Both should be rebound to validation332.
- **P4.** The F9 entry in `red_green` calls `f9_green_memory_guard.log` green, but that log ends "FAILED. 0 passed; 1 failed" at 4,452,912 KiB against the guard. It is a harness repair that now reaches its peak assertion. The README discloses this, but the memory guard is missing from the review README's Known red list.
- **P5.** `evidence/parity/django-413/summary.json` names source `1086daee4b`, while `per-rule.json` is the 332 run.
- **E1.** idiomatic-rust's `internal-crates` check exempts tests and fixtures (`tastemaker.toml:195` allowed-paths), which is why C1 passes TM. This is an open item for the tm-specialist.
- **Enforced boundaries limit.** The alias form of `ryo::languages` is not covered by idiomatic-rust 0.1.13. The CHANGELOG `f957c0632e` says so, and the steward decided none, pending the provider-alias branch. Whether each cited enforcement fires is the [leak review](leak-review.md)'s lens.

## Closed since 331

- The 331 Enforced-boundaries FAIL: tm 0.1.2 is rebuilt; shared-walk-owner, program-implementation-boundary and idiomatic-rust are active at 0.2.0, 0.2.0 and 0.1.13; tm validate is green; the whole tree has 0 findings.
- 331 P4.1 and P4.2: the matrix rows and the known red list are rebound to validation332. Row 1 remains stale (P2).

## Remaining manual judgment

- The TM receipt has 13 suppressed findings, where the agent file reads 0.
- The census is a static grep proxy written outside the review. The Relation::X row (24 to 25) did not reproduce: my filter gives 25 at both.
- The leak-repairs-b footprint is judged per class.
- This is a single review.

## Evidence JSON

`contract-review.json` sits beside this file.
