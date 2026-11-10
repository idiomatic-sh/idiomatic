# Leak review: claude/clippy-c

Source `6e24483050d9883057f88bd9ea7b5cf425c6d296`, base `877275be9e`. Commits: 7b02024c8e (execution_domain), 94957728f1 (execution_store), 6e24483050 (execution_runtime). 132 files, all in those three crates. The closure is those crates plus their importers and imports. No import, visibility or manifest line changed, so no crate outside them is affected. `.agents/agents/review-criteria.md` is not present in the integration tree, so no shared criteria were graded.

## Check 1: Proof where the contract lives (PASS)
No file under `benchmarks/` or `languages/` is touched. `execution_store/benches/performance/derived/bench.rs:72` is the crate's own bench. Its match became a let-else with the same panic.

## Check 2: Every enforcement claim fires (PASS)
- **Harness enumeration.** Every changed test or fixture keeps its assertion or panic:
  - `self_join_recogniser/service_test.rs`, `operator_collections_operators_source_matches_item_test.rs:76` and `graph_snapshot/mod.rs:117` are let-else with the same panic.
  - `partitions_test.rs:128` and `admission/test.rs:982` compare the same values through `try_from(..).expect`.
  - `outbound/derived/fixtures/contract.rs` still returns `DerivedFault::WrongOperation` for a non-equality view.

  No skip, filter, caught error, `continue` or weakened assertion was added.
- **Compiler plant.** A `CoverageState::Planted` variant at `completion/types.rs:16` gave E0004 at `frontier_coverage/service.rs:20`, the arm this branch named, and at 5 other exhaustive sites. Plant reverted; copy and target deleted.
- **Clippy lints.** `guard_shell.py` refused the reviewer's clippy run in the `git archive` copy, because clippy runs only in an integration worktree. I did not work around the hook. Instead, the capped clippy run of the real tree at the base (`/tmp/ryo-contract-validation334/clippy-capped-at-877275be9e.log`) reports at the exact lines the planned plants restored:
  - `as_conversions` at `trust/service.rs:92` and `replay/service.rs:49`
  - `wildcard_enum_match_arm` at `frontier_coverage/service.rs:18`
  - `missing_errors_doc` at `replay/service.rs:37`
  - `expect_used` at `workspace/mod.rs:297`

  Across the three crates that run reports 543 as_conversions, 251 wildcard arms, 212 missing_errors_doc and 38 expect_used. The lints are inherited through `[lints] workspace = true` (each crate's Cargo.toml:40-50) from `crates/ryo/Cargo.toml:139-155`.
- **Advisory.** A reviewer clippy run at the head is the probe that would turn these log counts into a reviewer-run count. It also needs the reviewer workflow and the hook to agree on where a reviewer may run clippy.

## Check 3: Boundaries hold across crates (PASS)
The diff adds no `pub`, `pub(crate)`, `use`, cfg, feature or manifest line. `canonical_graph_requirement` stays `pub(crate)` (`owner/graph.rs:112`). Both of its callers are in the same crate (`acquisition/graph/service.rs:72,182`).

## Check 4: One implementation and one representation per concept (PASS)
Every hunk rewrites an expression in place; no logic is copied into a test.
- Seven duplicate `EXPECT_*` message constants are deleted.
- The population entry is found once (`population/service.rs:84`), not re-fetched with `expect` three times.
- `link` uses `entry().or_insert_with` (`owner/mod.rs:233`).
- `release` keeps one `get_mut` slot (`lifetime.rs:27`).

## Check 5: Layer direction (PASS)
No import is added anywhere in the diff.

## Check 6: Test seams stay out of production (PASS)
No gate is added or removed. The new `expect_used`/`unwrap_used` expectations are on fixtures behind their existing gates. The one on production code, `Lease::drop` (`ledger/lease.rs:38-42`), adds a reason to a panic that was already there.

## Hunt list
- **Behaviour change: none found.** Each conversion keeps the refusal the lookup already had:
  - `map_or(None, ..)` where the lookup returned None;
  - `is_ok_and` where it was a predicate;
  - the same error variant where it returned a Result.

  On 64-bit, `u32`/`u64` to `usize` cannot fail, so no new error can happen where success used to. Specific cases:
  - **The new `ShapeMismatch` for `u8::try_from(populations.len())`** (`population/service.rs:57`, `owner/graph.rs:121`) never fires. `ids[..populations.len()].copy_from_slice(..)` on a 2-slot array panics first for any length above 2. Error order and behaviour are unchanged. Advisory: the panic is the real guard and the refusal is dead.
  - **`DemandAnswers` breadth stop** (`roots/value.rs:187-194`): `Ok(None)` when not stopped, as before. The former `expect` is now `ShapeMismatch`, but only on the path that used to panic.
  - **`InvocationWorkspace::acquire_derived`**: the two former `expect`s are now `WrongOperation` at the same position, before `admit_view`.
- **`examined` counts:** `GraphTraversal::direct` (`traversal/value.rs:31-39`). `begin()` zeroes `examined` and clears `reached`, and each neighbour adds one to both. The result equals the old `reached.len()`, so the value fed to `graph_counts.examined` (`roots/value.rs:185`) is unchanged.
- **Per-row cost.** `usize::try_from(u32)` on 64-bit is the unbounded `TryFrom` impl and always returns `Ok`, so the Err arms in the `cached_graph` row loops (`cached_graph/service.rs:218,281,302`) and in `replay::selected` compile away.
  - `direct` now pushes in a loop instead of `extend`. `reached` is a reused buffer, so the lost size hint matters only on first growth. Advisory.
- **`#[expect]` reasons.** All are true:
  - `OffsetList` and `NeighborList` are `Vec<u32>` and `VertexId` is `u32` (`derived/src/domain/vertices/types.rs:10-18`). The 11 kernel casts (sort 1, nearest 5, follow 1, neighbors 4) are lossless on 32- and 64-bit targets.
  - `usize` to `u64` (`graph_policy.rs:156`) and `u32` to `usize` (`identity.rs:30`) have no `From` impl, as the reasons say.
  - The route `elapsed_ms` u128 to u64 reason (584 million years) holds.
  - `pointer as usize` became `.addr()`, the same value.
- **File-length ignores: acceptable, advisory.**

  | File | Lines before | Lines after |
  | --- | --- | --- |
  | `operator/contract/service.rs` | 400 | 449 |
  | `operator/contract/types.rs` | 395 | 424 |
  | `workspace/mod.rs` | 383 | 411 |

  Each file was at or under the limit at the base and grows only from lint-required named arms and `# Errors` docs. The reasons are accurate. Splitting `types.rs`'s port traits from the contract was possible but is not required by any rule.

## Enforcement candidates
None. The findings are advisory and semantic.

VERDICT: PASS
