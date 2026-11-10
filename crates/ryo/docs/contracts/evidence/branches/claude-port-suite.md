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

VERDICT: PASS
