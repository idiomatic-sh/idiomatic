# Leak review: claude/port-engine (re-check of delta ffd0b1b134..cd9878320b)

Source `cd9878320b4f4b34d3844d254f2c5f3d76938337`, base `ffd0b1b1341e785c80cfd09019dc815b71c2a089`. Scope: the delta only. It touches three files, all on the crates the previous review covered; nothing else changed.

- `execution_runtime/.../distinct_integration/ordered_partial/test.rs`
- `execution_store/.../relations/number/service/issuance/service.rs`
- `execution_store/.../relations/number/service/test.rs`

The reviewed tree (`/workspaces/ryo-port-engine`) was the same before and after: HEAD cd9878320b, clean status. All plants ran in a detached scratch worktree at the same sha with its own target dir. Both were deleted afterwards.

## Check 1: proof where the contract lives (PASS)
All three files are in engine crates. Neither proof uses a product rule set or a corpus: the twin test uses a two-row SourceLocation table, and the store test (`number/service/test.rs:301-310`) uses a one-row computed table.

## Check 2: every enforcement claim fires (PASS)
Three plants, each reverted:

| Plant | Result |
| --- | --- |
| Recogniser's shapes dropped (`self_join_recogniser/service.rs:283` `Ok(_shape) => {}`) | `ordered_partial/test.rs:135` fails: "the plan is recognised and its ordered meaning entered". In the previous review this plant counted 0. The route_controls and join/release tests also fail. |
| Computed arm forced to refuse (`issuance/service.rs:53` `Computed => true`) | `ordered_partial/test.rs:134` fails with `"failed NotRun"` against the twin's complete table. |
| New ragged check removed (`issuance/service.rs:53-56` back to `Computed => false`) | `number/service/test.rs:336` fails with `Err(ShapeMismatch)` against `Ok(false)`. |

So the builder's unshown red is now shown. Without the check, the ragged source is not admitted silently. Instead it errors further down with ShapeMismatch, where it should be refused cleanly. The test tells these apart.

How the twin test now behaves:
- It reads `calls.ordered` for both sides (`ordered_partial/test.rs:110-130`).
- It asserts `ordered > 0` for `kept=[]` and `declined == 0` for `kept=[0,1]`, both for the whole read and for the read in part.
- It pins the whole answer with `starts_with("complete")` at line 150.

Nothing in the delta's harness can turn a failure into a pass: no skip, `continue`, caught error, filter or weakened assertion.

## Check 3: boundaries hold across crates (PASS)
- `admit_number` stays `pub(crate)` (`issuance/service.rs:7`).
- The store test is a `#[cfg(test)]` module (`number/service/mod.rs:24-25`).
- No visibility, manifest or feature changed.

## Check 4: one implementation per concept (PASS)
- The rectangularity predicate is still one predicate in one owner (`issuance/service.rs:39-58`).
- The twin test reuses the shared `run_read`/`MeaningCalls` harness.
- The store test drives the real `admit_number` and does not copy the predicate.

## Check 5: layer direction (PASS)
No new cross-crate import and no new layer edge.

## Check 6: test seams stay out of production (PASS)
- Both test modules are `#[cfg(test)]` (`ordered_partial/mod.rs:2-3`, `number/service/mod.rs:24-25`).
- The production change adds no seam.

## Enforcement candidates
All earlier candidates are carried forward. `twin-test-without-route-pin-22c0cc60` is now `instance_status: fixed`, and the other four stay `fixed`. The TM steward still decides each class.

VERDICT: PASS
