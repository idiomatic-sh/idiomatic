# Leak review: claude/w14-tm-import-reading (re-review 2)

Source `96b5545cb86ce05ed792810006665bd024c03a4f`, base `f67311c99057245e20ae4ed0da490bf4ee39e885`. The previous review, of `f1c9e1c968`, was PASS. This one covers `db77cdd326` and `96b5545cb8`, the responses to the Rust gate items F5, F6, F8, C1, C2 and C3.

## Check 2 Every enforcement claim fires: PASS

### The narrowed scopes query, run for real
I used `tm012c`, which carries the new query (checked in the binary), on a full archive of `96b5545cb8` with the spec-scenario workspace (crate `engine` denied):

| Probe | provider-imports |
| --- | --- |
| control `engine::run()` | line 1 |
| top-level alias, path in `mod inner` | line 4 |
| alias in one fn, path in a sibling fn | line 2 |
| alias in a `cfg(test)` mod, path outside | line 1 |
| alias in a closure body, path after it in the fn | line 4 |
| alias in a nested block, path after it | line 3 |
| alias in one `impl` method, path in a sibling method, all in `mod m` | line 5 |
| alias in `mod m`, path in nested `mod n` (with an empty `mod o` inside `n`) | line 5 |
| alias in a `const _` block, path in a sibling fn | line 2 |
| in scope: file top / same block / path two blocks below / after an empty sibling module | none |

**Why nothing is missed.** The query keeps the `stmt-list` that is the direct parent of a `use`, plus every `item-list`:
- every block a `use` can stand in holds its statements in one `StmtList`: fn bodies, nested blocks, closures, const, async and unsafe blocks;
- every inline module is present for the shut test.

A block with no `use` is never an import's innermost scope, and the edge's module test reads item-lists only, so dropping those blocks changes nothing.

### Bisect
`innermost` takes the latest-starting span that starts at or before the target and ends at or after it. Blocks and item-lists are either nested or disjoint, so that span is the innermost holder. The module-nesting and sibling-module probes confirm it.

### Memory guard: no longer vacuous
The test now requires exit 1 and at least one reported finding before it reads the peak (`cli/tests/replay/check.rs`). A refused run (exit 2) or a run that checks nothing cannot pass. It is not ignored, and it stays red at 4.37 GB until the engine-retention bead lands.

## Rows that bind no name: no change from base, for any provider
At the base (`f67311c990`), `keep(Relation::Names)` kept every `static-import` row with a target (`import_rows/service.rs:74-75`). The keep condition is again "target present", with `binding` optional. So the name rows match the base's for every language:
- Rust globs;
- JS side-effect and dynamic imports;
- Python star imports;
- Go blank imports.

Only a present binding can drop a use-site path, so a row without one never hides a path.

The intermediate states (`e196dbe503`, `f1c9e1c968`) dropped binding-less rows. That was a regression my earlier reviews missed; this commit closes it, and the `Unbound` case pins it.

## Checks 1, 3, 4, 5, 6: PASS
- **1:** the proofs are core cases and spec scenarios on fixtures.
- **3:** the boundary holds by scope in every probe.
- **4:** scopes are kept once per file, and each import's scope is found once.
- **5:** the change stays in tastemaker core's domain.
- **6:** no test seams in production.

## Enforcement candidates
Both are fixed:
- `use-site-binding-matched-file-wide-f7738c58`
- `provider-alias-resolved-across-blocks-3c609ce6`

VERDICT: PASS
