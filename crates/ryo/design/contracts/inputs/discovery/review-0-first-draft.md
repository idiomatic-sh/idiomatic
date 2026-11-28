<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a3a67b3e4b84a381f; copied from the session transcript -->

**Blocking** (evidence is at `2a207464c`; paths are relative to `crates/ryo/crates/` unless they start with `crates/` or `/`)

1. **`plan` and `ryoql` form a Cargo cycle.** The lowering takes the parsed program (`query_language/src/domain/plan_lowering/expansion/service.rs:25-30`, `plan_lowering/types.rs:7`) and the plan type names `AliasName` (`values/src/domain/finish_plans/expressions/types.rs:5`). `ryoql` in turn needs `plan` (`query_language/src/domain/kind_check/service.rs:8-9`, `inbound/use_cases/typecheck/keeper.rs:4-10`). Change: in section 3 move `values::{program, query_syntax, query_tokens}` from the `ryoql` row to the `plan` row, and delete "the plan lowering names no ryoql type" in section 2.
2. **`plan` and `ryoql` would import `outbound/language` and the run types.** `values/src/domain/finish_plans/types.rs:20` uses `runs::RuleId` (`runs/types.rs:4-6` imports `ryo_language`). `program/types.rs:192` uses `observations::ObservationName` (`observations/types.rs:4-5` imports `records` and `runs`). `TypecheckUseCase` takes a `QueryRequest` holding `hosted` and `observations` types (`query_requests/types.rs:25,27`). `plans/types.rs:374` and `query_language/src/domain/rendering/service.rs:21` name `ryo_language`'s `RenderContext`. Change: map `values` per item, not per module.
   - `RuleId`, `observations`, `hosted` and the readiness types `observations` takes from `records` land in `plan`.
   - `RenderContext` moves from `language/src/domain/semantics/types.rs:58` into `relations`.
   - `descriptions` (unassigned in section 3; used by 7 `finish_plans` files and `execution/src/inbound/use_cases/metadata/interface.rs:2`) lands in `relations`, its operator types in `operators`.
3. **`outbound/language` and `outbound/corpus` form a Cargo cycle, and `store`/`cache` are needed in pass 1 but created in pass 2.** `LanguagePort`'s types import `values::files` and `stored_facts` (`core/src/outbound/language/types.rs:12-13`). Section 4 sends `files` to `corpus`, which imports `ryo_language` (`core/src/outbound/corpus/adapters/ignore/adapter.rs:14-15`). `MemoryPort` (used at `sdk/src/resources/linter/lint/rows.rs:6,33`) and the stored and record types go to crates that section 1 defers to C1 and C3. Change: `values::files` goes to `outbound/language`; `outbound/store` is created in pass 1 with `MemoryPort`; section 4 names the pass-1 home of `records`, `stored_facts`, `stored_findings`, `stores`, `table_words`, `rows`.
4. **`execute` would import an inbound crate.** `executor/src/languages.rs:74` imports `ryo_core::inbound::use_cases::lint::coordinates::FindingCoordinates`, and section 3 sends core's lint interface to `inbound/lint`. Change: the mapping places `core/src/inbound/use_cases/lint/coordinates.rs` and `core/src/domain/finish` in `outbound/language`. With that, `application/` is sound: the executor's other imports (`plan_lowering`, `kind_check`, `SyntaxPort`, `ryo_matching`, coverage, trust) point at `domain` or move into `execute`.
5. **Step 1d is not a move.** The session is written against the SDK's own public types: `sdk/src/resources/linter/lint/session.rs:17` (`use crate::resources::linter as sdk;`, about 50 distinct `sdk::` names), `session.rs:34`, `session/modules.rs:2`, `lint/executor.rs:3-8`. `inbound/lint` cannot import the SDK. Change, one of:
   - 1d states that `lint/types.rs`, `errors.rs`, `executor.rs` and the `sdk/src/domain/*` modules they name move too and the SDK re-exports them. That changes `allow` at `crates/ryo/Cargo.toml:17-20`, `sdk/api.txt` and the `reexports` list at `crates/ryo/tastemaker.toml:124`.
   - 1d moves to pass 2.
6. **Step 1e moves less than it needs.** `link!` builds an SDK type (`sdk/src/resources/languages/macros/link.rs:7`; `Language` at `sdk/src/domain/language/types.rs:36`), and `sdk/src/resources/languages/contract.rs:7` re-exports `FindingCoordinates` from `core`. Change: add `sdk/src/domain/language` to 1e, and state that `Linter::new` then names an internal crate's type, which the `sdk-api` step refuses unless allowed.
7. **The landing gate in section 6 is not a gate, and the short check breaks on the new paths.** `/workspaces/ryo-poc-evidence/next/build/ports/gate-all.sh` is six lines comparing two fixed HTML-port images. `short-check.sh:53-54`, `build/tm-int/scripts/tip-pack.sh:9,11`, `build/one-engine/scripts/price.py:70` and `three-scores.py:16` hard-code `crates/ryo/languages/<lang>/benchmarks/linters`. `crates/tastemaker/benchmarks/project/live/images.sh:25` hard-codes `crates/ryo/crates/executor/benches/lint_image`. Change: section 6 lists the real steps (package tests, short check, three scores, timing pairs, `sdk-api`, a full Tastemaker check of `crates/ryo`), and 1a's file list includes these five scripts.
8. **"Anchor the globs" is rule authoring.** The unanchored `**/domain/**` globs sit in versioned definitions shared by all products, which `rename-tools-move.tsv` records as frozen: `tastemaker/rules/io-at-boundary/0.3.0/rule.toml`, `import-directions/0.1.0`, `no-reexports/0.6.0`, `tastemaker/specs/idiomatic-rust/0.1.16/spec.toml`. Each needs a new version. Also, `edges-apart` (`crates/ryo/tastemaker.toml:36`) forbids the command line importing the SDK, which section 2 requires. Change: take glob anchoring out of 1a into its own step, and add "remove `edges-apart`" to section 2.

**Non-blocking corrections**
- **Builder overlap:** by import pattern, 1b and 1c both edit 104 Rust files (49 in `core`, 16 in `cli/src`, 13 in `execution_domain`, 8 in `sdk`). 1d's 18 files overlap 1c in 8, 1e in 7, 1b in 5. All four edit `crates/ryo/Cargo.toml:2`, `crates/ryo/tastemaker.toml:8,9,35`, `sdk/Cargo.toml` and the lock. Have builders deliver rename maps that the integrator applies once.
- **1a:** it is a prerequisite only for crates that move whole; the five dissolving crates (116 of the 436 reference lines) have no folder to go to. The dry run prints only (`rename-plan.sh:2`), keeps each crate in its old parent (`:34`) and leaves 148 manifest paths as "recompute" (`:53`). It needs a folder column and an apply mode.
- **`tools/`:** `rename-tools-move.tsv` records that leaving the Ryo workspace loses inherited lints, 21 dependency versions and the release profile unless a workspace choice is made. The plan makes none.
- **Command line through the SDK (1b):** this rewrites five compositions and their rendering, so it is not a move. `cli/src/resources/query/run/rendering.rs:33` also implements `RowSinkPort`, which section 3 puts in `inbound/lint`.
- **Section 2, `edges/sdk` row:** "may import `inbound`" contradicts "picks the concrete outbound crates". Add `outbound`.
- **Section 3:**
  - `all()` is at `relations/src/domain/relations/service.rs:185`, not `catalog::service`.
  - "24 entries" is 23 (13 `crate`, 10 `nu-hex`).
  - `MatchingUseCase` is one 11-method trait covering regex, text globs and path globs (`matching/src/inbound/use_cases/matching/interface.rs:10-57`). Keep it whole in `operators` and let `corpus` import it.
  - Drop the re-exports at `execution/src/inbound/use_cases/metadata/interface.rs:6-8`, which would point `operators` at `execute`.
  - Every other trait name and method count matches.
- **Checked and sound:** no `core` use case imports another, so `validate`, `describe` and `lint` stay isolated. `language` itself imports only `relations`.
- **Gate gaps:**
  - Folder-keyed Tastemaker entries can silently match nothing: the `sdk` spec at `tastemaker.toml:124` has `count = { min = 0 }`, and there are 14 `internal-package-allowlist`, 5 `local-dependency-allowlist` and 2 `library-surface` entries. Compare matched files per entry before and after.
  - The SARIF identity runs on a frozen checkout, not the new tree.
  - `language/tests/ui/*.stderr` and `sdk/tests/ui/*.stderr` embed `$WORKSPACE/crates/relations`.
  - 247 doc-comment lines name dissolving crates.
  - `crates/tastemaker/Cargo.toml:249-262`, `crates/tastemaker/benchmarks/project/live/Cargo.toml:20-55` and `crates/ryo/release-plz.toml:24-93` name dissolved packages.
- **Scaffolding:** each new crate needs the `crate` spec's templated files. `core` has one `tests/bdd.rs`, one `benches/performance.rs` and 60 fixture files to split across five crates.
- **Section 6 bars:** they are not in `gates.tsv`. Its build 139 row reads dagster 19.89 s, Django 8.89, pandas 10.93, fastapi 2.19 and is marked not quiet. Cite the quiet source.

**Pass 1 in 3 to 4 hours: no as written.** The likeliest overruns are 1c (1,463 `ryo_values::` lines in 273 files, plus the item-level splits above) and 1d with 1e (they change the SDK's public surface). Yes if pass 1 is 1a, 1b and a 1c limited to the placements in items 1 to 4, with 1d, 1e, the command-line rewrite, glob anchoring and the `tools/` move cut to pass 2.

### Critical Files for Implementation
- /home/vscode/.claude/plans/squishy-sniffing-fountain.md
- /workspaces/ryo-poc-evidence/next/build/ports/stage2/rename-plan.sh
- /workspaces/ryo-poc-evidence/next/build/ports/short-check.sh
- /workspaces/idiomatic-public-benchmark/crates/ryo/tastemaker.toml (read at `2a207464c`)
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/values/src/domain/finish_plans/types.rs (read at `2a207464c`)
