<!-- a read-only reader's final report, 27 to 28 Nov 2026; task addd69addaa2ef011; copied from the session transcript -->

Paths: `P/` = `/workspaces/idiomatic-public-benchmark` at `2a207464c`, `C/` = `P/crates/ryo/crates`, `E/` = `/workspaces/ryo-poc-evidence/next/build`. Counts marked ~ come from name-based scans over git objects; nothing was built or run.

## Blocking problems
1. **The domain order is cyclic as written.** 22 production files in 19 modules that stay in ryoql name value types the plan sends to `plan`: `plan_parts` 80 uses/21 files, `plans` 22/16, `plan_orders` 3, `written_rows` 3, `lineage` 2. `C/values/src/domain/query_tokens/types.rs:3` and `query_bindings/types.rs:2` import `plan_parts` too, while `plan_lowering` imports `program`, `query_syntax`, `query_tokens` (`C/query_language/src/domain/plan_lowering/expansion/service.rs:25-30`). **Change:** only `values::finish_plans` goes to `plan`; `plan_parts, plans, plan_orders, written_rows, lineage, query_plans, query_bindings` go to `ryoql`, and the plan says ryoql then holds the step-plan type `Operation`/`PlanStep` and a lowering. Moving the 18 step-lowering modules to `plan` instead does not help: `syntax`, `observation_inputs`, `query_tokens`, `query_bindings` still need `plan_parts` below `plan`.
2. **Typecheck in `inbound/validate` breaks section 2 twice.**
   - It is ryoql's pipeline entry: `C/query_language/src/inbound/use_cases/typecheck/use_case.rs:10-18` calls `pub(crate)` functions (`assembling/service.rs:38`, `validation/service.rs:24`, `syntax/service.rs:286,336`, `syntax/constants.rs:25,221`) in `pub(crate)` modules (27 of 29 in `domain.rs`).
   - In pass 2 the session that moves to `inbound/lint` calls validate (`C/sdk/src/resources/linter/lint/session/entry.rs:186,200`), and `query run` builds `TypecheckUseCaseImpl` (`C/sdk/src/resources/query/run/schemas.rs:17-22`). That is lint importing validate, which `inbound-crates-are-isolated` forbids.
   - **Change:** typecheck with `keeper.rs` goes to `domain/plan` as its entry (all its imports are ryoql, plan, relations). `inbound/validate` keeps core's `ValidateUseCase`. Pass 2 states that the refusal-to-validate mapping stays in the SDK edge.
3. **`values` cannot be placed by the rule as worded.** "The lowest crate every user may import" is `relations` for every item. Read as "the lowest user's crate", over 639 top-level items:
   - 15 of 70 files need an item-level split (238 items live in them): `runs/types.rs` into 6 crates, `descriptions/types.rs` into 5, `plan_parts/types.rs` and `program/types.rs` into 3 each.
   - ~187 items (29%) have no production user outside `values` (records 34, stores 31, observations 25, plan_parts 18, table_words 18, stored_findings 10; `stores`, `stored_findings`, `query_plans` have zero outside files). The rule gives them no home, yet the plan says both "values is gone" and "nothing is forced into a crate".
   - Low crates stop owning one thing: ryoql receives 125 items from 13 modules (hosted, observations, plans, lineage); `outbound/language` receives `stored_facts` 4, `row_texts` 5, `rows` 2, `runs` 1, because `core/domain/finish` and `core/outbound/language` land there.
   - The one hard conflict, `plans/types.rs:374` naming `RenderContext`, is already handled.
   - **Change:** pass 0 delivers a per-item map for those 15 files; `values` stays as a shrinking remnant crate in pass 1 holding the ~187 until the owner decides; strike "values gone after pass 1".
4. **`plan_lowering` reaches outbound through two value files.** `plan_lowering/types.rs:6,8` imports `ObservationName` and `RuleId`. `runs/types.rs:4-6,31-35` names six `ryo_language` types, and `observations/types.rs:4` leads to `records/types.rs:13` (`FilePath`). **Change:** list `RuleId`, `ObservationName`, `ObservationColumn`, `BoundObservationInput` as domain items explicitly.
5. **There is no apply script.** `E/ports/stage2/rename-plan.sh:2` is "DRY RUN ONLY", renames folders in place (437 rows, 69 files), and line 76 says "lib names stay, so no `use` line changes". What does exist is the deletion loop: `fiximports.py`, `imp-fix.py`, `cascade.py` (deletes the fn or impl that names something gone), `fix-doctests.py`, `apply-plan.py`. **Change:** the script and its dry run on T are a pass-0 deliverable; those five tools are forbidden in pass 1.
6. **The `MatchingUseCase` split is a rewrite, not a move.** It is one trait of 11 methods (`C/matching/src/inbound/use_cases/matching/interface.rs:10-57`) with one impl, one 310-line contract and one bdd. `domain/matching/service.rs:3-4` imports globset and regex in one file, and the text-glob code uses globset too (`domain/globbing/service.rs:352`). **Change:** `matching` moves whole into `domain/operators` and `MatchingPort` plus its adapter into `outbound/corpus`; the split becomes a pass-3 cut. Also say the script deletes the upward re-exports at `C/execution/src/inbound/use_cases/metadata/interface.rs:6-8`.
7. **The direction rule as specified checks nothing, and cannot pass in pass 1.** `P/tastemaker/rules/layer-order/0.2.0/rule.toml:2` ranks by manifest directory name and says "Not seen: a package whose directory name no set holds". The plan's sets say `lint`, `plan`, `sdk`; the directories are `idiomatic-sh-ryo-lint` and so on. With real names it still fails: nine languages depend on the SDK until pass 2 (`P/crates/ryo/languages/python/Cargo.toml:34`) and the command line depends on the SDK (`P/crates/ryo/cli/Cargo.toml:22`). **Change:** sets use directory names; the pass-1 list keeps languages above the SDK; section 8.4 names the exact findings allowed at build 140.
8. **`landing-gate.sh` runs no package tests.** Lines 11-19 are identity, workloads, scores, pairs. Test lists are keyed `package<TAB>module::test` (`E/ports/landing/rehearsal-efdaf70c9-unexplained-tests.tsv:1`).
   - `E/ports/fork-b/crate-tests.sh:8,13,15` hard-codes `crates/ryo`, `-p idiomatic-sh-ryo-$c` and `[ -f languages/$c/tests/routes.rs ]`, so nine routes targets silently stop running after the move.
   - Seven package keys vanish: ~156 `#[test]` functions, ~141 doctests and 3 bdd targets read as both missing and new, so "regressions 0 / missing explained" in `gates.tsv` stops meaning anything.
   - **Change:** before pass 1, write a test step and a by-name comparison keyed through the rename map. Bar: mapped set identical, passed at least 1,768, unmapped missing 0.
9. **The gate scripts must read both layouts, not be "updated".** Hard-coded paths: `E/ports/short-check.sh:53-54` (the `git archive` failure is unchecked), `E/tm-int/scripts/tip-pack.sh:9,11`, `E/one-engine/scripts/price.py:70`, `three-scores.py:16,41`, and `images.sh` (`vec=$src/crates/ryo/crates/executor/benches/lint_image`). Old-against-new pairs need T priced by the same scripts. `short-check.sh:56` also picks the newest pre-switch image, which is build 133, not 139. The identity step itself is safe: its receipts and SARIF come from a frozen checkout.
10. **The Tastemaker work is about ten times what the plan lists.**
    - `P/crates/ryo/tastemaker.toml` has 470 path literals on 89 of 196 lines (333 under `languages/`), 41 package-name and 21 lib-name literals for the seven crates, and 308 `reexports` paths.
    - Templated files hard-code what moves: `P/crates/ryo/cli/build.rs:12` (`"../languages"`, feature name taken from the directory name, `ryo_core::` in generated text), `lint-allocation-test`, `lint-call-count-test`, `crate/0.13.2` `cargo-package`, Ryo's `language/0.4.18`.
    - 12 definition versions carry an unanchored layer-folder glob; the plan names 4.
    - **Change:** keep language folders named `python` and the command line folder named `cli`, or budget a new version per definition. The entry-by-entry before/after comparer is a pass-0 deliverable.
11. **Unassigned or unstated.** `RowSinkPort` has no pass-1 home though `core` is gone (called at `P/crates/ryo/cli/src/resources/query/run/rendering.rs:33`). Lib names are never stated (`ryo_query_language` 66 uses/28 files, `ryo_executor` 50/18). Section 2's `edges` row is false at build 140 (`cli/Cargo.toml:22-37` imports six internal crates).

## Question 1, the rest
- (a) After the two relocations nothing left in ryoql names `finish_plans` or `plan_lowering`. Only `domain.rs:15-16`, `kind_check/service.rs:8,9,31`, `typecheck/keeper.rs:4-10` and `typecheck/use_case.rs:20,150` do, and all of them move.
- (c) `TypecheckUseCase` callers: core `validate/use_case.rs:4-6`, cli `query/validate/composition.rs:26`, sdk `query/validate/composition.rs:39`, sdk `query/run/schemas.rs:17`. `kind_check` caller: `C/executor/src/check_session/kinds.rs:5`, which is downward. Nothing in `execute` imports inbound.

## Counts for the five dissolving crates (423 files)
- **Restricted visibility:** 155 `pub(crate)`, 14 `pub(super)`, 3 `pub(in)`; that is 105 items plus 44 `mod` lines. ~22 are named from a module bound for another crate, 20 of them in `matching` (2 if it moves whole). `query_language`, split four ways, has 334 items and 108 `mod` lines, ~57 crossing.
- **`$crate::` paths:** 4, the `#[macro_export] selects!` in `matching/src/cases.rs:32,34` and `values/src/cases.rs:33,35`. Two of these in one crate is a duplicate definition.
- **Feature gates:** 15 `cfg(feature)` lines, all `fixtures`; forwarding lists in `core`, `execution`, `execution_domain` and `sdk`.
- **Shared test support:** 5 distinct `cases.rs` variants used by 81 test files; `ryo_values::cases` in 7 files; `#[path]` harnesses in `core/tests/bdd.rs` (4), `query_language/tests/bdd.rs` (2, bound for two crates) and `core/benches/performance.rs` (2, bound for two crates).
- **Path rewrites:** `ryo_values` 1,515 uses/290 files (241 in doc comments), `ryo_core` 222/96, `ryo_matching` 48/24, `ryo_execution` 47/16, `ryo_execution_domain` 36/15, plus 398 `crate::` paths inside the five.
- **Other:** 1 orphan-rule impl (`values/src/domain/stores/types.rs:359`, which forces `stores` and `stored_findings` into one crate); 13 trybuild `.stderr` files; three `Cargo.lock` files to regenerate, because the bdd runners call `cargo metadata --offline --locked` (`query_language/tests/bdd.rs:26`).

## Question 5: references by file
- **Ryo:** `release-plz.toml` 7 package blocks and one `changelog_include`; `Cargo.toml` 25; `cli/Cargo.toml` 20 (the distribution test and bench attach at 103-110 by `../distribution`); 25 manifests with 147 path lines; `docs/README.md` 20; `docs/language-contract.md` 10; `asymptote.toml` 13.
- **Asymptote locks:** `P/crates/ryo/.asymptote/operations/*.lock`, 30 files with 1,073 path-keyed ratchet entries. The plan never mentions them.
- **Tastemaker product:** `Cargo.toml:249-262` (14 patch lines), `benchmarks/project/live/Cargo.toml` 25, `live/capture.rs` 3, `cli/Cargo.toml` 14.
- **CI and root:** `ci/` 14 files/71 references; `.github/workflows/release.yml:199`; `taplo.toml` 11; `_typos.toml` 9.

## Passes 2 and 3
- **Pass 2, most likely failure:** the author surface is a 200-line `pub use` hub (`C/sdk/src/resources/languages.rs`). The macros reach it as `$crate::languages::` (58 of 78 paths) and the nine languages use it 1,206 times, plus 188 macro calls. Moving it needs the hub inside `outbound/language`, which `no-reexports` allows only in the SDK, so the choice is new findings or about 1,200 deep-path rewrites.
- **Pass 3, most likely failure:** C2 "restores" a suite that stood at 265 passed, 15 failed at build 133 (`E/ports/fork-b/workspace-test-baseline-f0219b3b3.txt`). T's kernels take the executor's own `Plan`/`Op`/`E` and a column `Table` (`C/executor/src/finish.rs:148,490,1126,1144`).
- **Hidden ordering:**
  - C3 is blocked on C1, C4 and C6: `C/executor/src/check_session/replay/stored.rs:4-5` imports `Population, finish, Authority, project, Batch, Work`.
  - C5's branch edits `replay/batch.rs` (C6), `worker.rs` (C4) and five SDK session files that pass 2 moves.
  - C2 needs the table contract in a domain crate at or below `operators`; the plan names none.

## Kill criteria to add
- **Pass 0:** stop unless the checker exits 0 with every item of the five crates mapped to one crate, 0 upward edges and 0 unassigned; the dry-run apply on T prints 0 unresolved paths; the dual-layout short check passes on T.
- **Pass 1:**
  - An item census (fn, struct, enum, trait, impl, test, doctest, scenario; name and body hash after path normalising) at T equals the head. Stop on any missing item or changed body.
  - Stop on any new `fn`, `trait` or `impl`.
  - Non-path diff of at most 300 lines.
  - First compile: stop if privacy errors exceed the pre-counted ~80, or on any orphan error. That means the map is wrong.
  - Stop if any Tastemaker entry that matched files now matches none.
  - Stop if `tm` or `vec` changes size by more than 0.5%.
- **Pass 2:** stop on any removed public item in `api.txt`, any change under language `src` beyond import prefixes, or any exception to `inbound-crates-are-isolated`.
- **Pass 3, per cut:** stop if a case table is edited or deleted, if `.cols` outside `store` has not halved in two hours (C1), or if the pairs are not level.

## Lands in the stated time?
- **Pass 0 (1.5 h):** no. It lands as a module-level map plus the 15 split files, the remnant decision and the three tools from problems 5, 8 and 10; allow half a day.
- **Pass 1 (3 to 4 h):** no. It becomes plausible in one day with problems 1 to 3, 6, 7 and 9 changed and the language and `cli` folder names kept.
- **Pass 2 (3 to 4 h):** no. It needs the hub and lint-to-validate decisions first, and C5 landed before it.
- **Pass 3 (one day):** no. C1, C4 and C6 run in series before C3; C2 needs its own bar restated against the 15 known failures.

### Critical Files for Implementation
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/values/src/domain/plan_parts/types.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/tastemaker.toml
- /workspaces/ryo-poc-evidence/next/build/ports/stage2/rename-plan.sh
- /workspaces/ryo-poc-evidence/next/build/ports/landing/landing-gate.sh
