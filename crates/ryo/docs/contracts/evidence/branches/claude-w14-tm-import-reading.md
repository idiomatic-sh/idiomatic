# Leak review: claude/w14-tm-import-reading (re-review)

Source `f1c9e1c968d443ca0334182c40eacd0eb96a9d8d`, base `f67311c99057245e20ae4ed0da490bf4ee39e885`. The previous review, of `e196dbe503`, was FAIL: 2, 3.

## The fix
`import_rows/service.rs` `in_scope` (`:144-169`) drops a use-site path only when two conditions hold:
- the innermost block (`stmt-list`) or inline module (`item-list`) holding the binding `use` also holds the path;
- no inline module lies between them.

The scope rows are read from `syntax-tree`.

## Check 1 Proof where the contract lives: PASS
The proofs are the core unit cases plus three new spec scenarios (`spec.toml:350, :396, :442`), one per plant.

## Check 2 Every enforcement claim fires: PASS

**Plants re-run** with `tm012b` on a full archive of `f1c9e1c968`:

| Plant | provider-imports |
| --- | --- |
| control `engine::run()` | line 1 |
| `use … as engine` at top, `engine::run()` inside `mod inner` | line 4 |
| `use … as engine` in `fn a`, `engine::run()` in `fn b` | line 2 |
| `use … as engine` in a `cfg(test)` mod, `engine::run()` outside it | line 1 |
| in scope: file-level `use`, path in the file | 0 (correct) |
| in scope: `use` and path in the same block | 0 (correct) |

**Real tree** (`languages/rust/src/.../annotates/item.rs`):
- the control `ryo::query::ValidateRequest` reports `provider-imports`;
- the sibling-function plant reports 0. The cause is the provider leak below, not tm's reading.

## The provider leak (separate bead; does not block this branch)
`crates/ryo/languages/rust/src/domain/links/service.rs:232-249`: `through_uses` resolves a path's first name through any `use` with `each.enclosing == written.enclosing` (`:238`). That compares modules and ignores blocks, so a function-local alias rewrites same-named paths in sibling functions.

I verified it with this tree's provider (ryo 0.1.2) on a two-crate fixture. `source-dependency` targets:

| Path in `fn b` | Target |
| --- | --- |
| `ryo::query::ValidateRequest`, with `use crate::shared as ryo;` in `fn a` | `demo/src/shared.rs` (wrong) |
| the same path, no alias | `ryo/src/query.rs` |

The installed ryo 0.1.1 binary gives no use-site edge even for the control, so it could not show this.

Why it does not block this branch:
- tm's in-scope rule is correct and proven by the plants above.
- The fault is in the provider, which tm links from the registry at 0.1.1.

Required fix: `through_uses` scopes a binding to the block or inline module that holds the `use` (the rule tm now applies). Then release the provider and bump tm's pin.

## Check 3 Boundaries hold across crates: PASS
For tm's reading, the boundary now holds by scope. `ryo::link` and `ryo::reads` stay narrow: exact names, and both are `#[macro_export]` macros.

## Check 4 One implementation and one representation per concept: PASS
There is still one import reader; scopes are a new relation read, not a copy.

## Check 5 Layer direction: PASS
The change stays in tastemaker core's domain.

## Check 6 Test seams stay out of production: PASS
None.

## Ignore advisory: the reason holds; closed
A path tm drops is now one that the binding `use` is in scope for, so it names the same imported module. An ignore on that `use` exempts that one dependency; reporting each in-scope use of it would only repeat the same dependency. The scope fix also removes the over-reach flagged in the first review: an ignored `use` in one block no longer covers a same-named path elsewhere.

## Enforcement candidates
| Candidate | Instance |
| --- | --- |
| `use-site-binding-matched-file-wide-f7738c58` | fixed |
| `provider-alias-resolved-across-blocks-3c609ce6` | open |

VERDICT: PASS
