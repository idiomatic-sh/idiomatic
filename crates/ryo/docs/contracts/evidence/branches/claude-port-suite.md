# Leak review: claude/port-suite (re-check after round 3)

This review covers `f9a294ed5493ba65b458159fb9560f3dcd4e885f`, which is stacked on claude/port-engine `cd9878320b` (leak PASS). The round-3 delta is `bf89deded7..f9a294ed54`. It touches three test files in `crates/ryo/crates/execution/tests/` and `crates/ryo/tastemaker.toml:19`. The closure is unchanged from the previous review: execution, plus execution_runtime, execution_store, execution_domain and core's cross_file tests.

Probes ran in a detached worktree at the reviewed sha, with its own target directory. Every plant was reverted. The reviewed tree the port-suite worktree was not touched: HEAD stayed at f9a294ed54 and the status stayed clean, before and after.

## Check 1: proof where the contract lives (PASS)

Every new proof drives one of these, inside the execution crate:
- the public file session (`file_intents::answered`);
- the project phase;
- `run_batches`.

None rests on benchmarks/, languages/, a product rule set or a corpus.

## Check 2: every enforcement claim fires (PASS)

### Mutation table, re-run independently

I re-ran all 22 mutants from the builder's `mutations.py` at f9a294ed54, in my own worktree and target, against the `boundary_contract` binary. It has 153 tests, and the base run is green. The results match the builder's `final4` table exactly.

- **Caught** (each has at least one failure outside `interpreter::`): 01, 02, 03b, 04, 05, 06b, 07, 08, 09b, 10, 11b, 16, 17, 19.
- **Green:** 03, 06, 09, 11, 12, 13, 14 (equivalent), and 15 (unobservable).

### The four open items

1. **Mutation 6 / ordered existence: closed.**
   - S6 (`tests/completion/test.rs:232`) compares the recognised plan with its declined twin, whole and read in part, and asserts `affected == [None, Some(FileId(0))]`. It is green on the fixed engine.
   - Mutation 06b (every computed numeric source refused) fails S6: the recognised plan gives `failed NotRun`, the twin gives `complete`. That failure also shows that S6's plan really takes the recognised route.
   - Mutation 06 is equivalent at the seam. I probed S6 with the partition on Node (column 0), File (column 1) and Number (column 4), each whole and read in part. In all six cases, recognised equals twin, both on the clean tree and under mutation 06.
   - Why it is equivalent: in file scope, the input of an ordered region is either Complete, where both coverage functions answer Complete, or blocked and partial, where the blocked coverage `[None, Some(file)]` absorbs the difference. The recogniser declines CrossFile steps, so project scope never reaches it.
   - Advisory: as with mutation 12, the `ordered_output_coverage` branch in `publication/service.rs:108` is a distinction the seam cannot observe.
2. **S3 can now fail: closed.**
   - The left input of S3 (`completion/test.rs:103`) is now a step, `Select(Function)`, so the empty-result gate fires.
   - Mutation 03b fails S3 at `test.rs:118`: reads are 5 against an expected 4.
   - Mutation 03 is equivalent, as the previous review found with two plants.
3. **The population_reads control: closed.**
   - The control is `population_reads/test.rs:225`.
   - The ordinary population is evaluated in a project phase and gives reads (1, 1).
   - The observed population is evaluated through `request.observations(...).run_batches` and gives reads (2, 2). Both answer the same table, `[File(0)]`.
   - It is weaker than the control at b2b21fc043, which mixed the two demands in one phase. I accept the weakening, because that mix cannot be reached at the seam: `FrontierDemand` and `ProjectStage::evaluate_occurrences` are `pub(crate)` (`execution_runtime .../project_phase/types.rs:50`, `service.rs:112`). The restored control still fails if an acquisition leaks across phases, or if the counters stop counting.
   - The harness (`source_facts/fixtures.rs:155`) records stage errors as `Err` deliveries, and the test then panics. Nothing swallows a failure.
4. **T2 RelationOwner breadth: closed.** This item is scored under check 3.

### No silent passes

The round-3 delta adds no skip, `ignore`, `should_panic`, caught error, filter or weakened assertion. S3 now asserts more than before: the read count and the empty table.

## Check 3: boundaries hold across crates (PASS)

T2 (`crates/ryo/tastemaker.toml:19`) now admits `RelationOwner{,::fixture}`. I planted two violations in `tests/publication/store/test.rs`:
- `RelationOwner::routes`, as a path expression (`:39`);
- `RelationOwner::derived`, as a `use` (`:41`).

`idiomatic.arch/library-surface` fired once on each, and `test-imports` also fired once on each. The existing `RelationOwner::fixture` (`tests/publication/mod.rs:202`) stays clean. With tm 0.1.2, `tm check crates/execution/tests` reports 0 on the clean tree.

## Check 4: one implementation per concept (PASS)

Round 3 changes no production source. The new helpers `mixed_program_with` and `captured_in_file_0_of` (`tests/project_phase/mod.rs:57,302`) are test fixtures, not copies of production logic.

## Check 5: layer direction (PASS)

Over the engine, every `src/` hunk of the suite is a `#[cfg(test)]` module. Layer direction is the engine branch's, which passed its leak review.

## Check 6: test seams stay out of production (PASS)

The round-3 delta adds no feature, `cfg(test)` bypass or `pub` helper. `execution.rs:5-6` still gates `fixtures` behind `cfg(any(test, feature = "fixtures"))`.

## Enforcement candidates

`library-surface-admits-a-concrete-type-s-inherent-items-910da577` is carried forward with `instance_status: fixed`. The class still needs a decision from the TM steward.

## Advisories

- S6 does not assert on its own that the plan was recognised. Today, 06b proves that it is, and the engine's `ordered_partial` test asserts recognition. But if the recogniser regressed, S6 would pass vacuously.
- `ordered_output_coverage` cannot be observed at the seam (mutation 06), just like `contributed`'s Partial/Inapplicable split (mutation 12).

## Non-caught mutants

All paths are relative to `crates/ryo/crates/`, read at f9a294ed54. The runtime prefix `RT` is `execution_runtime/src/application/file_dag`. Each mutant below left all 153 `boundary_contract` tests green in my own re-run. A green run is not enough on its own, so each paragraph gives the argument for why no test at the seam can see the mutant.

**03, empty-gate coverage (equivalent; confirmed by probe).**
- The mutant drops `.with_coverage(completion.coverage(needs))` from the empty-gate publication (`RT/program/runtime/service/evaluation/service.rs:116`).
- The publication right above it, `inputs.publish(Region(id), &self.partition, &admitted, ...)` (`:104-113`), already gives the empty table the coverage of the admitted inputs. `publish_source` (`RT/program/facade_runtime/source_admission/publication/service.rs:23-127`) computes it through `contributed_proof` and `output_coverage`. That is the coverage the override would set.
- The probe: S3 (`execution/tests/completion/test.rs:103`) reaches this gate, because 03b turns it red at `:118` with 5 reads against 4. S3 asserts `Partial [Some(FileId(0))]` with a partial Comment read, and it stays green under 03.
- The previous round's two plants agree: both the root outcome and the frontier capture proof were unchanged.

**06, ordered output coverage (equivalent at the seam; confirmed by probe).**
- The mutant disables only the coverage branch at `publication/service.rs:108`. The `kinds` and `lineage` branches at `:59` and `:78` have a different prefix, so the mutation's match does not touch them. The ordered region then gets `output_coverage` in place of `ordered_output_coverage` (`execution_domain/src/domain/output_coverage/service.rs:374-396`).
- The probe: S6's plan, with the partition on Node (column 0), File (column 1) and Number (column 4), each whole and read in part. The recognised plan equalled its declined twin in all six cases, both on the clean tree and under 06.
- The argument: in file scope, the source region feeding an ordered region is either Complete, where both functions answer Complete, or blocked by a partial read. In the blocked case the region is empty, and the root outcome merges the blocked coverage `[None, Some(file)]`, which absorbs either answer.
- Project scope cannot reach the branch: the recogniser declines any step that is not PerFile (`execution_domain/src/domain/self_join_recogniser/service.rs:20`).

**09, early derived-view validation (equivalent; confirmed by reading).**
- The mutant adds a `validate_declared()` call before the partition loop (`RT/execution/lease.rs:35`). The original call at `:180` still runs.
- `validate_declared` (`RT/invocation/binding.rs:6-13`) can fail only on a fault of a derived view that was already built:
  - `axis()`, which goes through `AcquiredView::view`;
  - `dfs_view`, `cycles_view` and `ordered_view` (`RT/invocation/views/mod.rs:4-35`), which return the view's error, or `WrongOperation` when the acquired view has the wrong kind.
  - Both are internal faults: the view kind is fixed by the operator contract when the view is acquired.
- The mutant changes behaviour only when such a fault coincides with one of the exits between `:35` and `:180` (`lease.rs:96-178`):
  - `row_error`;
  - a `CheckedSpan` row skipped with `Ok(())` (`:102-112`);
  - the `CheckedText` `ShapeMismatch`;
  - a regex compile error;
  - a `graph_row` error.
- In those cases only the error's identity changes. No seam input builds a faulty derived view.
- Correction to the earlier round's wording: the failure can also be `WrongOperation` (a view of the wrong kind), not only an error from `port.view`. Both are internal faults, so the conclusion stands.

**11, member route in `dependency` (equivalent; confirmed by reading).**
- The mutant makes `dependency` (`RT/program/facade_runtime/source_admission.rs:90-200`, mutated at `:148`) resolve member 0's producer in place of member k's.
- Its result is only a scheduling hint, a `RegionId` to run first. It has two callers: `admission/service.rs:97` and `scheduling/service.rs:134`.
- The checks that can fail before `:148` (slot, rule ownership) are the same for every member.
- Acquisition resolves the correct member itself (`RT/program/facade_runtime/source_admission/acquisition/service.rs:186`). When that producer has not published, acquisition admits its source directly (`:214-226`, through `bind_rule` and `admit_source`).
- So the answer never depends on the hint. The member's supply (mutant 11b) is caught by `project_phase::members::test::each_member_of_a_file_binding_reads_its_own_producer`.

**12, Inapplicable before Partial in `contributed` (equivalent; confirmed by reading).**
- `contributed` (`execution_domain/src/domain/completion/service.rs:196-235`) has one production caller: `contributed_proof` (`RT/program/admission.rs:234-261`), called from `publish_source` (`publication/service.rs:23`).
- Both Partial and Inapplicable map to `ObservationReadiness::Waiting` (`RT/program/admission/admitted/service.rs:38-45`).
- Every production `publish` passes `ProducerRoot::Region` with `Representation::Computed`: `evaluation/service.rs:105,237` and `strategies/service.rs:33,72,172`. `Representation` has only the Computed variant (the irrefutable `let` at `publication/service.rs:127`).
- So every Waiting result has its coverage overwritten by `output_coverage` or `ordered_output_coverage` (`publication/service.rs:105-125`). The Partial/Inapplicable split in `contributed` never reaches an answer.

**13, latch keeps the last fault (equivalent; confirmed by reading).**
- The mutant makes `FailureLatch::record` (`execution_domain/src/domain/operator/contract/failure.rs:38-42`) overwrite the recorded error.
- `record` has two callers:
  - `FailureLatch::run` (`:26-36`), which returns early once a failure is held, so `record` runs only while the latch is empty;
  - `LeaseDriver::hold` (`execution_domain/src/domain/operator/contract/driver.rs:46-51`), which turns the error into `Fault::Interrupted`.
- Every operator propagates `Interrupted` with `?` as soon as it sees it, for example `operators/src/domain/equality_join/service.rs:34,55,70` and `ordered_existence/service.rs:26,31,52,57,69`. No operator swallows a driver fault: there is no `.ok()`, `if let Err` or `is_err()` on a driver result in `operators/src/domain/**/service.rs`.
- So `record` runs at most once per latch.

**14, re-running a published region (equivalent; confirmed by reading).**
- The mutant makes the `State::Published` arm of `poll` (`RT/program/runtime/service/invocation/service.rs:69`) fall through and run the region again.
- No path polls a published region:
  - `prepare_roots` drops terminal roots (Published, Failed, Retired) before it builds the queue (`RT/program/runtime/service/scheduling/service.rs:37-45`).
  - `invoke` returns a published region before it schedules (`:177-179`).
  - The queue marks every Published outcome Terminal (`:71`), so it never polls that region again.
  - The queue reaches a dependency only through `Suspended(dependency)` (`:62`), which admission returns only for a producer that has not published.
- The arm is defensive.

**15, retire skipped (unobservable; confirmed by reading).**
- The mutant skips `retire_producer` in cleanup (`RT/program/runtime/service/cleanup/service.rs:128-135`).
- `RelationOwner::retire_producer` (`execution_store/src/application/file_dag/relations.rs:249-255`) only calls `release_derived_producer` (`relations/producer/service.rs:118-137`). That function decrements derived-view pins and the byte and view counters in `DerivedOwner::release` (`derived/owner/lifetime.rs:9-58`).
- The resource limits never refuse a read. They govern retention only:
  - `prepare_graph_room` returns `Room::Full` (`derived/owner/graph_policy.rs:7-42`);
  - `try_graph_lease` then answers `Ok(None)` (`derived/owner/graph.rs:210-212`), and the graph is simply not retained (`derived/owner/graph_retention.rs:37,79`, `graphs/types/roots/value.rs:251,283`);
  - the `max_views` limit only skips caching a source view (`relations/source/service.rs:39`).
- So the mutant changes memory use and retention counters, which no seam answer carries. S15 (`execution/tests/publication/store/test.rs:20`) asserts that the publication of a retired producer stays readable, which holds both with and without the mutant.

VERDICT: PASS
