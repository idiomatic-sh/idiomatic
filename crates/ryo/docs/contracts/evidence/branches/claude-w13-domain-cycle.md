# Leak review: claude/w13-domain-cycle

- Source: `125637941db5d32a93cd5c82457d3c04d6129a2c`, base `b1e81f724206237fad67a42bfe1557fdb838b9e2`, one commit, 95 files.
- Scope: `execution_domain` (the domain moves), plus the three crates that import the moved paths: `execution_store`, `execution_runtime` and `execution` (one test path).
- tm, re-run by the reviewer in a `git archive` copy: `tm --root crates/ryo check --format json` gave **0 findings** and 6 suppressed, all `file-length` and unchanged from base.
- `cargo check -p idiomatic-sh-ryo-execution-runtime` passes in that copy, both with and without `diagnostic-row-progress`. The copy and its target directory were deleted.

## 1 Proof where the contract lives: PASS
The diff moves no proof to `benchmarks/` or `languages/` and asserts none from there.

## 2 Every enforcement claim fires: PASS
The branch cites one rule, `no-domain-concept-cycles`. Each ring was planted again in the real tree:

| Plant | Site | Findings | Reverted |
|---|---|---|---|
| `operating -> operator` import | `operating/types.rs:4` | 15 `no-domain-concept-cycles` + 2 `module-cycles` | yes |
| `self_join_recogniser -> executable_program` import | `self_join_recogniser/service.rs:4` | 3 | yes |

The 19 touched test files change call shapes only. No assert, expect or skip line was added or removed. The `holds_all` helper in `comparing/service_test.rs:36` is a thin wrapper over `holds_bound`, not a copy of it.

## 3 Boundaries hold across crates: PASS
Every moved item keeps its visibility, and none was widened. `portable_fault` and `derived_fault` (`operating/service.rs:44`, `:68`) stay `pub` because `execution_runtime` and `execution_store` call them. No path to an old home remains.

## 4 One implementation and one representation per concept: PASS
Each type is moved, not copied: every removed definition has exactly one new one, and `operator/contract/derived.rs` is deleted along with its placement in tastemaker.toml.
- `holds_all` and `holds_one` are gone. `shared_walk/service.rs:108` calls the one loop, `holds_bound`, and duplicates none of it.
- The `Texts` that `shared_walk` builds at `:96` replaces the identical projection in the deleted `holds_one`.

## 5 Layer direction: PASS
Both inversions are real injection from the parent.
- `kind_tests` (`kind_sets/service.rs:106`) now takes prepared kinds, matcher, symbols and source. The parent passes them at `shared_walk/service.rs:261`.
- `holds_bound` is handed its texts, bindings, matchers and `DecisionWorkspace` by `shared_walk` (`:108-122`).

No child calls back up. Production `kind_sets` and `comparing` no longer import `OperatorContext`.

## 6 Test seams stay out of production: PASS
The diff adds no test seam. The feature-gated progress items moved with their `cfg` gates intact.

## The questions in the brief
- **True owner, or a grab-bag?**
  - Owner moves: `OperatorContext` and `Emit` to `operator`, the context's consumer and the top of the ring. `DfsResources` to `derived_views`, the OrderedDfs view's parameters. `PrimitiveSemantics` to `operating`, beside `SemanticLookup`. The fault mappings and `written_result` to `operating/service.rs`, the owner of `OperatingError`.
  - `ReaderMultiplicity` in `operator/types.rs` fits that file's existing role, "shared execution accounting vocabulary", beside `StepPlace` and `PlanIdentity`.
  - `IndexedRows` in `table_reach` is defensible, but `table_model` would be the stronger owner (see the second finding).
  - `operating/types.rs` is **not** becoming a catch-all. It shrank from 139 to 138 lines, and everything it gained is operator I/O vocabulary. Its module doc (`:1`) is stale.
- **Inversions:** real. See check 5.
- **Is `shared_walk` duplicating logic?** No. It calls `holds_bound` with one comparison per predicate, as the old `holds_all` did.
- **Zero-allocation fast path:** preserved. `holds_one_bound` returns on `equal_to_text` (`comparing/service.rs:186`) before it dereferences the `LazyScratch` (`:191`).
  - The change also makes the runtime predicate path lazy. At base, `predicates.rs` passed `&mut *self.state`, which deref-coerced to `&mut SelectScratch` and initialised the scratch on every call.
  - No unit test asserts the property (see the first finding).

## Advisory findings, for the PM to file as beads
1. **Zero-allocation equality fast path has no direct assertion.** The fix is a `comparing/service_test.rs` case asserting `select_scratch.0.get().is_none()` after Equal-only rows, and `is_some()` after a Glob. It is a missing test, not a rule signature, so it has no `enforcement_candidates` entry.
2. **Two hash-chain probes over `KeyIndex`, pre-existing.** They are `table_reach/borrowed.rs:112-150` and `table_model/service.rs:169-210`. Candidate `duplicate-key-index-probe-09dbebd5` (new rule): only `table_model` may read `KeyIndex.heads` or `.next`.
3. **Stale module doc** at `operating/types.rs:1`.
4. **Test reaches up.** `kind_sets/types/vocabulary/value_test.rs:5` builds a full `OperatorContext`. Candidate `test-imports-a-parent-concept-7c7a3416` (strengthen `no-domain-concept-cycles`): apply the cycle rule to `*_test.rs` files.

VERDICT: PASS
