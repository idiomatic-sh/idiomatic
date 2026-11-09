# Rust gate: checkpoint 332, delta pass

- Reviewed source: `2991a41cd2b10a176f3da435660ef32983aaa8a9`.
- Delta: `c3639e10a6..2991a41cd2`. The previous pass, at `c3639e10a6`, was PASS 7/7.
- The delta carries four things:
  - B1 and B4 (`5e7ca6fb96`);
  - the tm 0.1.2 import-reading merge (`0f2fa35a12`, with branch commits `55e5fd3520`, `7fb164654f`, `e196dbe503`, `e2c66a218a` and `f1c9e1c968`);
  - the `std::sync::atomic` allow-list entry (`883f0d6dd6`);
  - the record and the evidence.
- Evidence read: the diffs, the files they touch, and the integrator's logs `gatefix_tastemaker_*.log` in the validation332 directory. Under the review definition I ran no cargo.
- Tree: HEAD stayed at `2991a41cd2` for the whole pass. The only working-tree changes are this file and its JSON.
- Already known and being fixed, counted as FAIL here as you asked: the cold memory guard (finding F5) and the reporting test (F6). The go/api samples are F7.

**Verdict: FAIL.** Items 3 and 6 fail; items 1, 2, 4, 5 and 7 pass.

| Item | Verdict |
| --- | --- |
| 1 Ownership and API shape | PASS |
| 2 Compiler over tests | PASS |
| 3 Hot path | FAIL (F5) |
| 4 One representation per concept | PASS |
| 5 Standard crate over home-grown | PASS |
| 6 Tests | FAIL (F6, F7, F8) |
| 7 Rust fit | PASS |

## 1 Ownership and API shape: PASS

- **B1 is applied.** `crates/tastemaker/crates/core/Cargo.toml:33` requires `idiomatic-sh-ryo-sdk` 0.1.2.
- **B4 is applied.** The seal's doc comments now say exactly what it enforces: every `MatchingUseCase` implementation, in any crate, gets `PatternSemantics` through the bridge, and no type implements it directly (`crates/ryo/crates/execution_domain/src/domain/matching/types.rs:11-27`).
- The new `Scope`, `ScopeKind` and `Name.binding` (`crates/tastemaker/crates/core/src/domain/imports/types.rs:76-99`) are Tastemaker domain rows.
- `sdk/api.txt` is unchanged.
- Advisory C1: `crates/tastemaker/cli/Cargo.toml` still asks for SDK 0.1.1, and for the language crates at 0.1.1.

## 2 Compiler over tests: PASS

- `Relation::Scopes` is a new variant, so `query` and `keep` cover it in exhaustive matches.
- `ScopeKind` is an enum, mapped from the two kind texts in one place (`import_rows/service.rs:102-113`).

## 3 Hot path: FAIL

**F5 (fix pending).** Tastemaker's cold check of `crates/ryo` peaks at 4,491,136 KiB against its guard of 1,331,200 KiB. It is red only with the import-reading branch's core changes (`validation.json` `tastemaker_workspace`; `gatefix_tastemaker_without_tm_branch.log`).

The scope logic I was asked to look at explains it:

**(a) The scopes rule emits far more rows than it uses. This is the likely cause of the memory.**
- `SCOPES` (`crates/tastemaker/crates/core/src/domain/import_rows/constants.rs:39-41`) is `select id, kind, file, span from "syntax-tree" where glob_any(kind, "{stmt-list,item-list}")`.
- That emits one row for every `stmt-list` and every `item-list` in the tree: every function body, block, `impl` body, `trait` body and inline module. On `crates/ryo` that is tens of thousands of rows, where only the scopes around a `use` and the inline modules matter.
- `glob_any` on `kind` is not a kind equality. Only a leading run of total kind equalities narrows the engine's dispatch (`execution_domain/src/domain/shared_walk/service.rs`), so the rule tests every node of every file.
- Each emitted row comes back as a lint answer with all its columns, and is then copied into a `Scope` that owns its own `RepoPath` (`import_rows/service.rs:108-112`).

**(b) `in_scope` repeats the same work for every edge. This costs CPU.**
- For each use-site edge, `imports()` runs `in_scope` against each name whose binding matches.
- `in_scope` scans all of the file's scopes twice: `filter` plus `min_by_key`, then `any` (`import_rows/service.rs:147-169`).
- It finds the innermost scope of the same `use` again for every edge, so the cost is O(edges × scopes) per file.

Required change:
1. Emit only what `in_scope` reads. The innermost scope of a `use` is the parent node of that `use` item, so query the `use` items' parents (one row per import) plus the item lists that belong to an inline `module`. Select by kind equality, for example two selects joined by a union, instead of `glob_any`.
2. Key scopes per file without one `RepoPath` copy per row.
3. Compute `around` once per name, and find the inline modules around an edge by bisecting scopes sorted by start.
4. Re-measure with the cold memory guard (`check::checking_crates_ryo_cold_stays_under_its_memory_guard`).

## 4 One representation per concept: PASS

- `Scope` projects the engine's `syntax-tree` answer into Tastemaker's own row types, as `Name`, `Package` and `Owner` already do for their relations.
- The constants left unused by the query's early returns are deleted: `EDGES_RELATION` and `EDGE_KEY`.

## 5 Standard crate over home-grown: PASS

No new dependency. The only lock change is Tastemaker's own version, which goes from 0.1.1 to 0.1.2.

## 6 Tests: FAIL

**F6 (fix pending). The reporting test is red, and the cause is a real false negative.**
- `crates/tastemaker/crates/core/src/application/reporting/service_test.rs:492` reads 0 findings where it expects 1 (`gatefix_tastemaker_workspace.log:7960`).
- The cause: `keep()` now keeps a `static-import` row only when it has both cells, `if let (Some(written), Some(binding))` (`import_rows/service.rs:73-82`). The test's answer carries `target` and `span` but no `binding`, so its import is never read.
- Production hits the same drop. The JavaScript provider gives an unknown binding to a side-effect import (`import "x"`) and to a dynamic `import(..)` (`crates/ryo/languages/javascript/src/domain/syntax/service.rs:272`, `:291-292`). Those imports would silently stop reaching every import rule.

Required change:
1. Make the field `binding: Option<ModuleName>` and keep every row that has a `target`.
2. Match a use-site path's first name only against `Some(binding)`.
3. Add an `import_rows` case for a row without a binding (advisory C2).

**F7 (pending a later delta). Two go/api replay rows went red through the patch.**
- `replay samples each_row_of_each_sample_is_fixed_and_restored`, both its "inside a repository" and "as a repository of its own" rows, fail at 332 through the local ryo crates the patch links. They are green at 331.
- My second pass missed this, because only `tm_import_cli` had been run under the patch (its B6 advisory). Required change: the fix the PM has scheduled, then the replay green.

**F8. The two tm fixes have no red-then-green record.**
- `55e5fd3520` and `e2c66a218a` change production behaviour, but their commit messages name no test, and `validation.json` `red_green` still holds 30 entries.
- Their cases exist in `crates/tastemaker/crates/core/src/domain/import_rows/service_test.rs`:
  - `a_path_at_a_use_site_through_a_name_an_import_binds_is_that_imports`
  - `a_path_at_a_use_site_from_the_package_root_is_an_import_of_its_own`
  - `a_path_in_an_inline_module_sees_no_import_written_outside_it`
  - `a_path_outside_the_block_of_an_import_sees_no_binding_of_it`
  - `a_path_outside_the_inline_module_of_an_import_sees_no_binding_of_it`
  - `a_path_in_the_block_of_an_import_is_that_imports`
- None is shown failing without its fix. Required change: reverse each fix's production hunks, run the cases red, restore and run them green, then add the entries. The F6 repair also needs a test, which C2 supplies.

## 7 Rust fit: PASS

- The delta adds no `unsafe`, no `#[allow]`, and no `unwrap`, `expect` or `panic` in production code.
- `size()` maps an unplaced scope to `usize::MAX` on purpose, so it never wins as the innermost scope (`import_rows/service.rs:148-151`).
- The `std::sync::atomic::*` allow-list entry (`crates/ryo/tastemaker.toml:205`) is justified. The providers' production code already counts with it: the python `parse/contract.rs`, and the ruff parser's `adapter.rs` and `semantic.rs`.

## Advisories

No bead ids here. Each advisory becomes a bead, or is dropped with a reason.

- **C1. The cli manifest still asks for 0.1.1.** `crates/tastemaker/cli/Cargo.toml` requires `idiomatic-sh-ryo-sdk` 0.1.1, and the ryo language crates at 0.1.1, while core requires 0.1.2. Cargo resolves both to one version, so nothing breaks today, but the manifests disagree. Fix: raise the cli's ryo requirements to 0.1.2.
- **C2. No case covers a row without a binding.** Add an `import_rows` case for a `static-import` row with no `binding` cell. It is the test F6's repair needs.
- **C3. The `in_scope` scope rules have gaps.** In Rust, a `use` inside a block also binds for the items nested in that block, and a glob import (`use a::*`) binds no single first name. `in_scope` handles the first only through the block's span, and the second never drops anything; both outcomes are conservative. If the reading should go further, state the rule.
