# Leak review: claude/w14-tm-import-reading

Source `e196dbe50300fb85144bc19da48d3f81b507acde`, base `f67311c99057245e20ae4ed0da490bf4ee39e885`.

## The leak
The new drop at `crates/tastemaker/crates/core/src/domain/import_rows/service.rs:172-179` matches against bindings file-wide. If any import anywhere in the file binds a use-site path's first segment, the path is dropped:

```rust
.is_some_and(|first| brought.iter().any(|name| name.binding == *first))
```

The check ignores the scope the import binds in. An import bound in one scope therefore hides a same-named extern-crate path in another scope, where Rust resolves the name to the crate.

**Plants.** I ran the built tm 0.1.2 (`scratchpad/w14tm/tm012`) on a full archive of `e196dbe503`. The workspace is copied from the spec scenarios: `deny` by default, allow `crate::shared::*` and `ryo_language::*`, with a denied crate `engine`.

| `demo/src/domain/model.rs` | Findings |
| --- | --- |
| control: `pub fn work() { engine::run(); }` | 1 |
| top-level `use crate::shared::contract as engine;` + `mod inner { pub fn work() { engine::run(); } }` | **0** |
| `fn a() { use crate::shared::contract as engine; … }` + `fn b() { engine::run(); }` | **0** |
| `fn b() { engine::run(); }` + `#[cfg(test)] mod tests { use crate::shared::contract as engine; … }` | **0** |
| `use crate::shared::*;` + `fn b() { engine::run(); }` | 1 (glob imports are unaffected) |
| `self::engine::Collect` through the import | 0 (judged as written, as before) |

**Required change.** Drop a use-site path only when the `use` that binds its first segment is in scope at that path, and no nearer scope rebinds the name. In scope means the `use`'s enclosing module or block span holds the path's span. In every other case, judge the path as written. Add three scenarios to `provider-imports`, one per plant above (nested mod, sibling fn, `cfg(test)` mod), each expecting the finding at the `engine::run` line.

## Check 1 Proof where the contract lives: PASS
The proofs are the core unit cases and the spec scenarios, both on fixture workspaces.

## Check 2 Every enforcement claim fires: FAIL
`provider-imports` reports 0 on the three plants above.

The existing tests can fail:
- The two `use_site` unit cases discriminate:
  - removing the drop leaves the site import in `contract::Collect` → fails;
  - an over-broad drop removes the site import from `crate::contract::Collect` → fails.
- Each of the four scenarios discriminates in the same way.

None of them puts a binding out of scope.

## Check 3 Boundaries hold across crates: FAIL
The import boundary is unenforced in the cases above: provider code reaches a denied crate with no finding.

The allow-list entries `ryo::link` and `ryo::reads` (`crates/ryo/tastemaker.toml:205`) are narrow:
- each is an exact name, not a glob;
- both are `#[macro_export]` macros (`sdk/src/resources/languages/macros/link.rs:10`, `reads.rs:53`), so nothing can be reached below them.

## Check 4 One implementation and one representation per concept: PASS
There is one import reader. `Name` gains a `binding` field, read from the existing Names relation column.

## Check 5 Layer direction: PASS
The change stays in tastemaker core's domain.

## Check 6 Test seams stay out of production: PASS
None.

## Advisory
A `use` silenced by an in-source ignore now also silences every use-site path through its binding. That fits judging the import once, but the ignore reaches further than its line.

## Enforcement candidate
`use-site-binding-matched-file-wide-f7738c58`, mechanism: strengthen `idiomatic.ryo/provider-imports`.

VERDICT: FAIL: 2, 3
