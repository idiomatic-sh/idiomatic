# Leak review: claude/port-engine

- **Source:** `c23d7fa7596b8af6b03d9381aa68c31b3340e4b3`
- **Base:** `40e49021520d4392d29df28ee5bd2fef739e5d37`
- **Scope:** execution_store, execution_domain, execution_runtime, execution, core, and cli's replay fixture. The SDK is in the closure but has no change.
- **Method:** the diff and the touched files were read at the landed sha. The plants were made in a `git archive` copy:
  - the compile_fail bodies were compiled verbatim with `cargo check` in a probe crate inside that copy;
  - T1 was run with tm 0.1.2.
- **What the reviewed tree did:** `/workspaces/ryo-port-engine` stayed at `c23d7fa759` and stayed clean before and after the review.
- **Where the deviations came from:** the bead's notes, as `bd show` prints them, hold no list of 8 deviations. So the deviations judged here are the ones the commit messages record (see "The deviations").

## Check 1: Proof where the contract lives (PASS)
- No file under `benchmarks/` or `languages/` is touched.
- The new proofs sit with their owners and use hand-built plans:
  - `execution_domain/src/domain/executable_program/service_test.rs:248-363`;
  - `completion/service_test.rs`;
  - `execution_runtime/.../preparation/service/demands/service_test.rs`.

## Check 2: Every enforcement claim fires (FAIL)
**Plants that fired:**
- T1 (`crates/ryo/tastemaker.toml:17`) fired on all 3 core plants: a stale forward `PublicationSlot`, a grouped `{RuleIndex as _, ReadinessProof}` and a glob `interface::*`. Each plant reported 1.
- T1 did not fire on the negative `PhaseStop::Unbound`, so the `{,::*}` arm works.
- The store's E0624 doctest (the control) catches `computed` made `pub`.

**Three ways a failure passes:**
1. **The observer contract cannot fail.** `execution_runtime/src/outbound/observer/fixtures/contract.rs:81-86` runs every row with a closure that returns `Ok(())`, and `Value = ()`. `ObserverFake::totals` always answers `FedTotals::default()` (`fixtures/fake.rs:21-23`) and still passes.
   - **Required change:** the contract asks the adapter and compares its answers, as the digest contract does: `Value = bool`, and the totals after the reports equal their sum. The fake then keeps real totals.
2. **The counter seals are bundled.** `program/file_session/interface.rs:23-31` puts three calls in one `compile_fail,E0599` doctest, and `program/project_phase/interface.rs:30-38` puts two in one.
   - Putting back `pub fn walk_counts` on `FileSession` left the body failing to compile, with E0599 on `readiness_counts` and `counts`, so the doctest still passes. The plant counted 0.
   - Putting back `pub fn state_counts` on `ProjectPhase` also counted 0.
   - **Required change:** one doctest per removed method.
3. **A test was weakened.** `core/src/application/cross_file/service_test.rs:97,112` replaced the measured `done.fed.len()` with the literal `0`, and `:176` expects `[2, 0, 10]`. That column cannot fail.
   - **Required change:** drop the column, or measure fed through an `ObserverFake` given to `begin`.

## Check 3: Boundaries hold across crates (PASS)
- **Sealed:** these are `pub(crate)` and their forwards are deleted:
  - `PublicationSlot`, `BoundInput`, `ReadinessProof`, `InputReadiness`;
  - `PhaseOccurrence(s)`, `FrontierDemand`, `RuleSelection`;
  - `GenerationLease`;
  - 20 `RelationOwner` methods.
- **No other route:**
  - `ObservationStage`'s fields are `pub(crate)`.
  - `ObservationStage` no longer implements `ProjectRootExecution` (pinned by an E0599 doctest).
  - The stage protocol methods are `pub(crate)` inherent methods.
- **The interface:** `execution/.../execution/interface.rs` forwards exactly T1's list.
- **T1's coverage:** T1 carries the general entry's engine libraries for core, sdk and cli, so the general entry's new exclusion of those three loses no coverage.

## Check 4: One implementation and one representation per concept (FAIL)
- **The protocol was moved, not copied.** Core's `bind`, batch loop and `BatchOccurrenceBinding` are deleted. `ObservationStage::run_batches` (`program/observation_stage/service.rs:35-176`) is the one loop. `RequestDerived::compile` left the store, and its walk lives once in `preparation/service/demands`.
- **But "does this run count the rows" now has two representations:**
  - the dead enum `Counting` (`execution_domain/src/domain/operator/types.rs:33`), still forwarded (`execution/.../execution/interface.rs:59`) and listed in T1's surface (`tastemaker.toml:17`);
  - `Option<Arc<dyn ObserverPort>>` (`core/.../lint/use_case.rs:96`), which replaced it.

  No production file reads `Counting`.
- **Required change:** delete `Counting`, its forward and its T1 surface entry.

## Check 5: Layer direction (PASS)
- The observer is a runtime outbound port. It imports only `execution_domain`, and the application reaches it through `ProgramServices::observer` (`facade_runtime/types.rs:141,167`).
- `file_program` imports `observation_schedule`, never the reverse.
- `execution_store/src` names no `executable_program` item.

## Check 6: Test seams stay out of production (PASS)
- These are all behind `cfg(any(test, feature = "fixtures"))`:
  - `FileSession::fixture_*` (`file_session/service.rs:126-170`);
  - `ProjectPhase::fixture_*`;
  - `FileRun::collected_counts` (`file_execution.rs:278`);
  - `RelationStore::fixture` (`relations/interface.rs:119`);
  - `observer::fixtures`.
- No manifest changed, and every `fixtures` feature is still enabled only through dev-dependencies.

## Hunt findings (from the brief)
**H1 (blocking). The §2.2 claim is false on one success path.**
- `LintUseCaseImpl` hands its observer to `begin` (`core/.../lint/use_case.rs:269`).
- `reused` → `FileVisits::read_alone` (`core/.../file_analysis/streaming/service.rs:287-330`) runs each changed file through the same `services.run_file`. Each of those runs reaches `rest_of_rule` → `session.observe` (`request_execution/service.rs:377`).
- Before this landing, `read_alone`'s `tables.fed` was dropped: `ReadAlone` carries only records, unknowns and skipped, and nothing absorbed it.
- So on a counted run whose decision is `Check(changed)`, `features` and `work` now add the read-alone visits. That run ends `Ok`, so the report is wrong.
- Production passes `None`, so the production output is unchanged. The counted replay and fixtures are affected.
- **Required change:** do not observe the read-alone visits. For example, `read_alone` runs over services begun without the observer, or core passes the observer only to the main visit.

**H2 (advisory). A refusal was deleted.**
- Core's file-path check `counted.len() != fed.len()` → `ShapeMismatch` (formerly `file_analysis/evaluation/service.rs:122-124`) is gone, together with its `fed.get_mut(rule) → NotRun`.
- It can be reached only if a rule's metadata step count differs from its `original_steps` count. Not settled in one reading.
- **The probe that would settle it:** assert the two lengths are equal in `preparation/service/metadata`.

**H3 (advisory, design §2.5). The owner-identity refusal is gone.**
- `ObservedPublication` no longer checks `Rc::ptr_eq(owner, proof.owner)`. Before, `AdmittedInput::publication(owner)` checked it at `validate_transfer_readiness` and at the `BoundKey::Current` population.
- `ResidentGeneration::live` is all that remains. In today's flows it is equivalent, because one project owner serves every batch.

**H4 (advisory). `ObservedPublication::readmitted` re-reads the rows.**
- It calls `owner.rows(&publication, unavailable)` and admits again (`observed_publication/service.rs`), where before it passed the held `AdmittedInput`.
- A source publication whose rows need the reader would now refuse with `NotRead`. Observation producers publish shared or compact project tables, so in today's flows it does not.
- Not settled. **The probe that would settle it:** an alias over a file-source publication.

**H5 (advisory). The F1 guard does not exercise the risk it was written for.**
- `step_origin_place_equals_the_logical_lookup` builds a program with no observations, unions or aliases. So `compile_unions` and `aliases::resolve`, the reason the guard exists, never run under it.
- A stale `place` would make the `CheckOnly` path refuse `NotRun` in production.

**H6 (advisory). The counting observer outlives a request.**
- The counting observer belongs to the use case, so a `LintUseCaseImpl::counting(o)` reused for two lints reports cumulative totals. Before, the totals were per run.
- No in-tree caller reuses one today.

**H7 (advisory). The observer adapter fails silently.**
- The counting adapter silently drops rule `>= COUNTED_FED`.
- On a poisoned lock it drops a report and answers empty totals (`counting/adapter.rs:19-25,44-50`).

**The order and text of refusals:**
- `run_batches` keeps:
  - the selection and finish of unselected rules;
  - the restore;
  - per-batch `bind` in declaration order (frontier_readiness → reserve → bindings);
  - the arenas read once per batch;
  - the submitted-to-declared mapping;
  - the delivery loop with `delivering`;
  - `finish(rule)`, then the guard's refusal, then `finish_stage`.
- A binding error now reaches core as `PhaseStop::Unbound` and is mapped to `CrossFileError::from(error)`, the same conversion `bind`'s `?` made.
- The install now uses the bound slot, not the slot `complete` returned. They are the same `Copy` value.
- `logical_counts` keeps its checks in their order.
- No error text changed.

## The deviations (from the commit messages)
| Deviation | Forced? |
|---|---|
| `ObservedPublication.generation` added beside the four fields the design lists | Forced: the sealed facade gives no generation of a `Publication` without its owner. |
| `SemanticNeed` → `rule_intent`, not `prepared_execution` | Forced: there is a preparation ↔ prepared_execution cycle. |
| The facade has 12 operations, not 10 (`admit_number` and `normalize` stay separate) | Forced: their signatures and lifetimes differ. |
| `ObserverPort` has `totals` beside `fed` | Forced by the port template's contract. |
| `ObservationBatches` flattened, with `BatchOccurrence` in place of a tuple and of `scope: PhaseRules` | Forced: `PhaseRules.arenas` cannot be supplied, because the stage reads its own arenas per batch. |
| The `ResidentGeneration`, `PublicationBody`, `PublicationHold` and `OccurrenceBinding` names | Forced by the unique-names rule. |
| T1 carries the runtime outbound arm and `BatchOccurrence` | Required for coverage parity. |

No deviation was found unforced.

VERDICT: FAIL: 2, 4
