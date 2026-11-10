# Leak review: claude/cp334-doc at 36c1524724

Base dee086491c, source 36c1524724ec225fb3578aa88529fe0ba2139ed2. Three doc-only commits (79b6940e92, 625cf08ffe, 36c1524724).

## Brief checks
- Diff contents: 594 insertions, 0 deletions; every added line is a `///` doc line except one reasoned ignore, `// ignore: idiomatic.arch/file-length ...` at `crates/ryo/crates/execution_domain/src/domain/observation_schedule/service/link.rs:1` (file now 429 lines). No code changed.
- Value examples, sample of 8: rows 1 `contributed` (merge of two Complete is Complete), 6 `schedule(&[])` is Independent, 7 `schemas` on an observation-free rule yields empty schemas and bindings, 8 `request` Independent, 9 `link` empty, 18 `validate_spans` on empty rows is Ok, 33 `ReplayClosure` equality, 37 `counting()` totals after one `fed`. All of them call real code and assert its result. Advisory: about a dozen struct-literal V rows (2, 3, 4, 11, 14, 15, 17, 27, 29, 32, 41, 51) only read back a field they just set. The approved table specifies exactly that, so this is not a defect.
- Stubs: exactly 9, matching table rows 5, 22, 23, 24, 25, 28, 34, 35, 36. Each states its reason in a comment, and the reasons check out: `with_matchers` is `pub(in crate::domain::executable_program)` (value.rs:84); `FileSession.parts`, `ProjectPhase` fields and `ObservedPublication` fields are `pub(crate)`; `ObservationBatches` and `PhaseRules` borrow `&dyn FactsFinish` / `&dyn FactsWorker` that only a run supplies.
- Doctests, own target, behind the build lock, `--all-features --doc` one package at a time: domain 23 passed (+21), runtime 19 standalone + 12 merged (+18), store 13 + 4 merged (+12). 0 failed, total +51, which matches B1 + 51.
- `run.py` `doc_problems` at the sha: one line, `crates/ryo/crates/execution_store/src/application/file_dag/relations/interface.rs:40 pub trait RelationStore`, which belongs to the other branch.

## Check 1 Proof where the contract lives: PASS
No contract is asserted from benchmarks/ or languages/: all 51 new doctests live on the items they document in execution_domain, execution_runtime and execution_store src (git diff dee086491c..36c1524724 --stat: 24 files, all under those three crates' src/), and none reads a product rule set or corpus.

## Check 2 Every enforcement claim fires: PASS
The diff cites no Tastemaker rule, compile_fail probe or lint (nothing to plant). The doctests cannot pass vacuously: no ignore/no_run/text/should_panic fence (102 bare ``` fences = 51 blocks), V and S blocks assert results of real calls (e.g. crates/ryo/crates/execution_store/src/application/file_dag/relations/generation/service.rs `same` asserts Err(NotRead) across two owners; crates/ryo/crates/execution_runtime/src/outbound/observer/factory.rs asserts counting() totals). Run at 36c1524724 with --all-features --doc per package: domain 23 passed (base 2, +21), runtime 19+12 passed (base 1+12, +18), store 13+4 passed (base 1+4, +12), 0 failed, exit 0. run.py doc_problems at the sha leaves exactly one line: crates/ryo/crates/execution_store/src/application/file_dag/relations/interface.rs:40 pub trait RelationStore (the other branch's item).

## Check 3 Boundaries hold across crates: PASS
No visibility, manifest or import changes: every added line is a /// doc line except the ignore comment at crates/ryo/crates/execution_domain/src/domain/observation_schedule/service/link.rs:1; no '-' lines in the diff. Store examples reach `fixtures::program_derived_port` and `RelationOwner::fixture` only through the crate's own dev-dependency with features=["fixtures"] (crates/ryo/crates/execution_store/Cargo.toml [dev-dependencies]), so no fixture path is opened to production.

## Check 4 One implementation and one representation per concept: PASS
Doc examples add no type, loop or data shape; the T stubs and the local `struct Uniform` in crates/ryo/crates/execution_domain/src/domain/source_partitions/types.rs (ProducerPartitions example) are doctest-local illustrations, not copies of production logic.

## Check 5 Layer direction: PASS
No item moved or changed signature (doc-only diff); examples import only public paths of the documented crate and its existing dependencies (ryo_values, ryo_relations, ryo_execution_domain), no inbound or application item newly crosses a crate.

## Check 6 Test seams stay out of production: PASS
Doctests compile only under cargo test; the fixture-using store examples (rows 42-49) rely on the self dev-dependency with `fixtures`, which a production build never enables (execution_store/Cargo.toml features comment: only a dev-dependency or the crate's own `fixtures` switches it on). No new feature, cfg(test) bypass or pub helper.
Advisory: the published docs of 8 store items show a `fixtures` path (`RelationOwner::fixture`, `fixtures::program_derived_port`). Users on docs.rs see test-support API in the examples. The approved S form chose this, and it does not open a production path.

Method note: the shell guard requires a git worktree for `cargo test`, so I ran the doctests in a temporary detached worktree at the sha instead of a `git archive` copy. Its target directory was deleted and the worktree removed afterwards. The reviewed tree stayed at HEAD 36c1524724 with an empty `git status --porcelain` before and after.

VERDICT: PASS
