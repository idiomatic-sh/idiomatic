# Rust gate, checkpoint 333: FAIL (6 of 7 PASS; item 2 FAILs)

- Source reviewed: `6af005e9f749d03192fb87f68db577ef42944ce5`. The prompt gave `6af005e9f7c6c1e70ae7d98d7a3a5d6f6e6a52c0`, and git has no such object. Its 10-hex prefix `6af005e9f7` matches the reviewed HEAD.
- Base: `dcea2ee7df7c02711f42c332cbc403ff7b84a034` (the landed 332).
- Scope: perf step 2 skip rules (`8eeb39f8ed`), the noqa fragment rewrite (`555076902b`), the provider alias fix (`6e7265eaac`), the integrator's merge repairs and splits (`28efeb7bf0`, `5339ed5df3`), the removal of asymptote.json (`5e2aad3bf1`), and the census and record commits.
- Tree: HEAD was `6af005e9f749` at the start and `226de88f02` at the end. Three docs-only commits landed during the pass (`3d1e6d91a4`, `623ef2feaa`, `226de88f02`), and `.agents/agents/contract-review.md` and `README.md` had uncommitted edits. `git diff --name-only 6af005e9f749` names only those two files and the two `evidence/branches/claude-w14-isolation-lens.*` files. No `.rs`, `.py` or `.toml` file changed, so every file this review read is byte-identical at the reviewed sha. The PM decides whether that is enough or whether the pass is discarded under the "tree changed" rule.

## 1 Ownership and API shape: PASS
- The diff touches no public SDK surface: no `api.txt`, no `crates/sdk` and no `Cargo.toml` change.
- `presence::service::of_graph` builds its tables by value. `for_regions` consumes `GraphPresence` into the region table (`execution_domain/src/domain/executable_program/presence/service.rs:280-308`).
- `FilePresence` is a plain field of `FileRun` that keeps its buffers across files (`execution_runtime/.../file_execution/presence/service.rs:147-174`).
- The new `RefCell` fields (`absent`, `interior`, `refused`) extend `RuntimeState`'s existing `&self` design (`runtime/service.rs:169-177`). They add no new pattern.
- `NotesError` becomes `ProjectError::Context` through a typed `From` (`languages/rust/src/outbound/notes/errors.rs:11-18`).
- Advisories A1, A2, A4 and A5 cover the remaining clones and the leftover `Arc`.

## 2 Compiler over tests: FAIL
**F1.** `crates/ryo/crates/execution_domain/src/domain/operator/contract/service.rs:230`: `failure()` ends with `_ => RowFailure::Never`.

Why it matters:
- The skip-rules design is sound only if every operation declares which input rows can fail it. If one does not, its region publishes the empty table on a file where the full run reports a row error.
- With the wildcard, a new `Operation` variant compiles as `Never` by default.
- Only a hand-maintained test census defends this today: the `Raiser` lists and `every_fault_has_a_verdict` (`presence/service_test.rs:517-560`, `:684-705`). The census is exhaustive over `OperatingError`, but not over `Operation`. A new operation that raises an existing error passes unless its author adds it to a raiser list.
- The same change already uses exhaustive matches for the twin declarations: `presence()` (`service.rs:174-191`) matches every `OperationKind`. `invalid_patterns` (`presence/narrowing.rs:96-124`) names every `Operation` and its doc says "so a new one that carries a literal pattern cannot be left out".

**Required change:** replace the wildcard with the explicit list:

```rust
Operation::Project { .. } | Operation::Extend { .. } | Operation::Union { .. }
| Operation::Intersection { .. } | Operation::Difference { .. } | Operation::Antijoin { .. }
| Operation::GroupCount { .. } | Operation::GroupDigest { .. } | Operation::Capture { .. }
| Operation::Rank { .. } | Operation::Words { .. } | Operation::Closure { .. }
| Operation::Cycles { .. } | Operation::Rows { .. } | Operation::Hosted { .. } => RowFailure::Never,
```

This is a compile-only change with no behaviour change. The existing presence `service_test` stays the behaviour proof, and the compiler becomes the guard. The other new wildcards are safe: `node_narrowing` (`narrowing.rs:66`) and `tree_output` (`projection.rs:124`) default to the conservative answer (no narrowing, no tree).

## 3 Hot path: PASS
- The per-row presence loop allocates nothing. It reads rows into the reused `cells`, retains open tests in place, and takes `scratch` and `memo` from the file (`file_execution/presence/service.rs:278-337`).
- The runtime uses `FxHashMap` (`state/service.rs:190`).
- The fold's std `HashMap`s (`presence/types.rs:145-160`) run once per request at compile, not per file or per row.
- Removing the per-request reader memo for a per-file walk (`file_demand/service.rs:157-230`) is a measured perf trade under the perf rule: `evidence/perf/p7/after.json` shows byte-identical output on the one-file, 100-file and Django workloads.
- Per-file allocations of O(regions) remain: `AbsentRegions::none`, `reached()`, the walk's `counts`/`queued`/`pending`, and `Arc::new(table)`. They are per file, not per node or per row, so they do not fail this item (A2, A3).

## 4 One representation per concept: PASS
The diff removes duplicates rather than adding them:
- Column-origin propagation has one definition, `output_origins` (`operation_schema/projection.rs:134-233`). `trust/reads.rs:71-82` calls it instead of its own copy.
- `tree_output` (`projection.rs:103-124`) replaces the inline copy in `file_program/service/operators.rs:96-99`.
- `RegionCount` moved to `executable_program/types.rs:12` and is deleted from readiness.
- `AbsentRoots` is a newtype over `AbsentRegions` (`file_demand/types.rs:118`), not a second store.

## 5 Standard crate over home-grown: PASS
- No dependency change.
- `AbsentRegions` (`file_demand/types.rs:136-166`) is a 30-line bitset, the same kind of glue as the existing `ReadinessBits` (`values/src/domain/records/types.rs:307`). No bitset crate is in `crates/ryo/Cargo.lock`, and adding one for this would not fit.

## 6 Tests: PASS
- **Provider alias:** both fixes name their cases in their messages (`a33a4c13f1`, `6e7265eaac`). The red-then-green record is at `crates/ryo/docs/contracts/README.md:78`:
  - compile reds without the notes' `scope` field;
  - a behaviour red on the after-plant built at `28efeb7bf0`;
  - a tm plant red on `0f2fa35a12`;
  - green at `cbdc70adad` (9 / 0).
- **`28efeb7bf0`:** a compile repair with no behaviour change.
- **Skip rules and noqa rewrite:** these follow the perf rule (`evidence/perf/p7/after.json`, `evidence/perf/noqa-fragments/after.json`).
- **Premise changes, each with a stated reason:**
  - `remaining_is_computed_once_per_selection` is removed with the memo it measured (`c84c355dfb`).
  - `target_supplement.observed_row` now accepts the whole-inventory summary. It still refuses any other row that is not `FAIL` / "not selected for this invocation" / 0 cases (`target_supplement.py:17-27`), and `5339ed5df3` states the reason.
  - The expected values 40→47 and N999→RUF100 were re-executed, not read off.
  - The two UP035/UP036 reds stay red and are named.
  - The hook test `test_asymptote_record_is_validated_when_present` went with the mechanism it tested (`5e2aad3bf1`).
  - The step-8 skip-gate relaxation to 87.76% is recorded as a give-back in `57d3675517`.
- No skip, ignore, caught error or wall-time assertion is added. The `continue` in `presence_service_test.rs` records `Err(NotRun)` for a non-run intent and is not a pass.

## 7 Rust fit: PASS
- No `unsafe` and no production `unwrap`, `expect` or panic.
- The one `#[allow(clippy::too_many_arguments, reason = ...)]` is test-side (`execution/.../conformance/fixtures.rs:47-50`).
- The compile-time twin check is a `debug_assert!` (`executable_program/service.rs:245-251`).
- Lookups return `OperatingError::NotRun`.

## Advisories (open-source hygiene; beads for the PM to file, or drop with a reason)
- **A1, per-file double ownership clones.**
  - `walk/service.rs:237` passes `AbsentRoots::decided(decided.clone())` and then reads `decided`.
  - `file_execution/presence/service.rs:140` passes `mark_interior(interior.clone())` and returns `interior` only for a trace count.
  - Fix: store once in `RuntimeState` and read back through the existing `presence()` accessor, or have `mark_interior` return the count.
- **A2, reader table built twice per file with absences.** `bind_intents` calls `activate_file_roots(&readers, AbsentRoots::default())` (`walk/service.rs:200`), and `activate` builds the table again when presence finds absences (`:237`). Fix: build the default table lazily, or only when the presence gate is off.
- **A3, `Representation::empty` takes an owned schema it only borrows.** `admission.rs:211-216` takes `Result<Vec<CellKind>, _>` and only reads `&schema?`. The new `absent` strategy clones `output_kinds` for every absent region of every file (`strategies/service.rs:77`), and so does the blocked strategy (`:38`). Fix: take `Result<&[CellKind], &OperatingError>`.
- **A4, `RootReaders::Selected` keeps an `Arc` with one owner.** With the per-request memo gone, `Arc::new(table)` (`state/service.rs:213`, `runtime/service.rs:158`) has a single owner. Fix: hold `RemainingTable` directly.
- **A5, project contract clones each parsed note for `facts_in`.** `note.clone()` at `languages/rust/src/inbound/contracts/project/contract.rs:147`. Fix: collect `notes` by value and let `facts_in` consume them, or have `facts_in` borrow the note.

The JSON lists each advisory as `open item: <title>`. Their bead ids are for the PM to fill in when filing.
