# Census trend: change amplification, checkpoints 318 to 333

The question: "is our architecture getting meaningfully better and easier to extend or modify on each cycle?" The owner added: adding semantic analysis, or tuning one perf strategy, must be "very isolated".

## Sources and method

| Checkpoint | Source measured | How it was chosen |
|---|---|---|
| 318 | `c043a6dc68` | validation318 source (contracts README) |
| 321 | `4f0a08dd7d` | validation321 final source (records.md:90) |
| 324 | `14a8352416` | checkpoint 324 commit |
| 328 | `4a982fa0a5` | validation328 source |
| 331 | `340808449f` | validation331 source |
| 332 | `781653e221` | head of `claude/checkpoint-332` |
| 333 | `e0a566da62` | head of the checkpoint 333 tree. It adds 20 commits over 332: noqa fragment rewrite, p7 skip rules, provider alias, t611 |

- Every number comes from static `grep` over `git archive` copies. I built nothing and ran nothing. The copies are deleted.
- A production file is a `.rs` file under `crates/*/src`, `cli/src` or `languages/*/src`. Excluded: `*_test.rs`, `test.rs`, `fixtures*`, `tests/`, `acceptance`, `benches/` and `benchmarks/`. This is the audit's rule, plus benches and benchmarks.
- "Engine" means every crate except the languages and the two benchmark crates.
- The first three change types are counted by proxy: the files that name one existing member, as the audit did for row 7.
- The GroupCount regex also matches `group-count` and `group_count`. That gives 43 files at 332, where the audit counted 35 (it skipped the kebab-case strings and the sdk). The trend is the same either way.

---

## 1. Add a plan operator kind (proxy: `GroupCount`)

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Prod files naming `GroupCount`/`group-count` | 43 | 40 | 40 | 40 | 42 | 43 | **45** |
| ... crates | 6 | 6 | 7 | 8 | 8 | 8 | 8 |
| Prod files naming ≥8 distinct `Operation::` variants (exhaustive matches) | 22 | 21 | 21 | 21 | 23 | 24 | **27** |
| Prod files naming any `Operation::X` | 99 | 63 | 63 | 63 | 67 | 68 | 69 |
| ... crates | 4 | 5 | 7 | 8 | 8 | 8 | 8 |
| `Operation::` arms in core `domain/visit/service.rs` | 26 | 26 | 26 | 26 | 26 | 26 | 26 |

- **Verdict: worse.** The kernel fold (321) cut the files that name `Operation::` from 99 to 63. Since then the exhaustive matches have grown from 21 to 27.
- The crate splits (324, 328) spread the same files over more crates: 5 → 8.
- query_language added 5 match sites (21 → 26) at 331 to 332: span-semantics and `refusing`.
- There is still no operator registry. The 27 match sites at 333 are:
  - core `visit` (1);
  - execution `observing` (1);
  - execution_domain (13);
  - operators (1);
  - query_language (9);
  - values (2).

## 2. Add a relation that rules can read

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Per-file semantic relation, proxy `binding_read`: prod files / crates | 22 / 13 | 23 / 13 | 23 / 13 | 23 / 13 | 23 / 13 | 23 / 13 | 23 / 13 |
| Required relation, proxy `static_import`: prod files / crates | 34 / 13 | 34 / 13 | 34 / 13 | 34 / 13 | 34 / 13 | 34 / 13 | 34 / 13 |
| Project-scoped semantic fact, proxy `ProjectNamespaceMembership` + `project_context`: prod files / crates | 74 / 18 | — | 84 / 18 | 84 / 18 | 84 / 18 | 84 / 18 | 84 / 18 |
| ... of which benchmark crates | 25 | — | 32 | 32 | 32 | 32 | 32 |
| Engine prod files naming a specific `Relation::X` (outside relations): files / occurrences / crates | 29 / 170 / 4 | 24 / 154 / 5 | 24 / 154 / 7 | 24 / 154 / 8 | 24 / 154 / 8 | 25 / 155 / 9 | 25 / 155 / 9 |

**Verdict: flat, no improvement in any cycle.** A per-file relation costs 23 files in 13 crates. A project-scoped fact costs 84 files in 18 crates. The engine's special-casing of named relations went down once (29 → 24 files at 321) and has since spread over more crates (4 → 9), because the crates were split.

## 3. Add or edit one rule

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Files per rule edit (`rules/<CODE>/query.ryoql` + `cases.toml`) | 2 | 2 | 2 | 2 | 2 | 2 | 2 |
| What reruns: `stored.rules != rules → Decision::Whole` (core `domain/storing/service.rs:54-55`) | all files × all rules | same | same | same | same | same | same |
| Per-rule copies of the noqa `inline-pattern=` | 806 | 806 | 806 | 806 | 806 | 806 | **0** |
| noqa directive grammars (suppression fragments vs RUF100 `source-matches`) | 2 | 2 | 2 | 2 | 2 | 2 | **1** |
| Rules importing a suppression fragment | 414 | 414 | 414 | 414 | 414 | 414 | 394 + 10 + 9 |

- **Verdict, rule assets: better at 333.**
  - The noqa rewrite collapsed 806 per-rule copies to 0 and 2 grammars to 1 (audit rows 3 and 10).
  - Plan size dropped from 112,063 to 44,813 nodes.
  - The rewrite itself had to edit 479 asset files, which is evidence of the amplification it removes.
- **Verdict, store: unchanged.** Editing one rule still reruns every file against every rule (a cold Django run, 293.7 s at 332).

## 4. Change the per-input runtime lifecycle (admission, publication, release; audit row 1)

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Prod files naming the lifecycle types* | 57 | 56 | 61 | 61 | 61 | 61 | 61 |
| ... directories | 40 | 40 | 46 | 46 | 46 | 46 | 46 |
| ... crates (+ domain `operator/contract/lease.rs`) | 1 | 1 | 2 (+1) | 2 (+1) | 2 (+1) | 2 (+1) | 2 (+1) |
| Prod files naming store concrete types (`RelationOwner`, `InvocationWorkspace`, `GenerationLease`, `InputAdmissions`) | 57 | 47 | 51 | 51 | 51 | 52 | 53 |
| Engine prod files naming admit/Admission | 237 | 236 | 240 | 240 | 242 | 242 | 244 |

\*The lifecycle types: `AdmissionProgress`, `SourceInputs`, `AdmittedInput`, `InvocationResources`, `InputAdmissions`, `release_inputs`, `ResourceLease`, `InvocationWorkspace`, `inspect_available`, `evaluate_once`, `GenerationLease`.

**Verdict: worse.**
- The same roughly 60 files now sit in 46 directories across 3 crates. At 318 it was 40 directories in 1 crate.
- The store/runtime split (324) cut through the concept. Runtime calls 42 concrete store methods (audit row 2), and no port was added.
- A lifecycle change used to be one crate. Now it is a cross-crate change made in the same commit.

## 5. Change a runtime internal: tests in other places that break (audit row 9)

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Test files outside the runtime/store home naming their internal types† | 45 | 55 | 48 | 48 | 49 | 50 | 49 |
| `execution/tests` files / those importing runtime, domain or store crates or internals | 101 / 0‡ | 133 / 46 | 103 / 46 | 103 / 45 | 104 / 50 | 105 / 50 | 105 / 50 |
| Test files in other crates naming `ryo_execution_runtime` | n/a | n/a | n/a | 122 | 128 | 127 | 129 |
| Test files in other crates naming `ryo_execution_domain` | n/a | n/a | 131 | 131 | 143 | 154 | 157 |

†The internal types: `FileRunInput`, `FileSession`, `FileInput`, `RanRule`, `RuleFile`, `SourceRows`, `FileRules`, `FileFinished`, `EmittedRoot`, `KindSupport`, `SemanticSupport`, `CaptureContext`, `RuleSelection`, `AdmittedInput`, `InputAdmissions`, `RelationOwner`, `InvocationWorkspace`.

‡At 318 the runtime was inside `execution`, and its tests reached it through the use-case forward.

**Verdict: flat to worse.** The audit's 50 cross-crate test files are unchanged since 324. Tests in other crates that name the domain crate grew from 131 to 157.

## 6. Add semantic analysis for Python (name resolution, types, call targets), traced at 333

**Isolation target:**
- `languages/python`: new item directories under `domain/relations/collections/relations/<fact>/` plus its contract;
- `crates/relations`: one item directory and one catalog row per fact;
- nothing else: no engine crate, no other language, no test outside those two crates.

That is 2 crates. A per-file fact costs about 3 to 4 files; a cross-file fact costs the same plus one provider port.

**Must change today, per fact.** The precedents are `binding_read` (per-file) and `ProjectNamespaceMembership` (cross-file).

| Area | Must change | Evidence (333) |
|---|---|---|
| Relation registry (relations crate) | 6 to 7 files per relation: 5 lists plus the item directory | `catalog/service.rs:27` (catalog row); `relations/collections/relations.rs:79` (mod); `capabilities/types.rs:21` (`OptionalCapability` variant); `relations/service.rs:304` (capability → relation map); `relations/constants.rs:195` (capability list); `collections/relations/<fact>/{item,constants}.rs` |
| Every other language (9 crates) | a written-out `fn <fact>() -> absent(..)` in each `inbound/contracts/parse/contract.rs`. The catalog generates one required trait method per relation, with no default (language-contract.md:3-6) | `languages/go/.../parse/contract.rs:351`, `languages/rust/.../parse/contract.rs:295`, and 7 more |
| SDK | re-export of the expected row | `sdk/src/resources/languages/contract.rs:27` |
| execution_domain (partitioning) | an exhaustive `relation_partition` arm per relation (CurrentFile vs Project) | `execution_domain/src/domain/source_partitions/types/source_partitions/value.rs:225-293` (`BindingRead` :245, `SourceDependency` :291) |
| Cross-file: language crate | Project reads manifests only. There is no link/resolve use case: README "a language crate has the `parse` use case only, with no `link` use case and no semantic relations" (docs/README.md:23). The one cross-file semantic fact (`ProjectNamespaceMembership`) put a Python-namespace concept into the shared crate | `crates/language/src/domain/project_context/{service,types}.rs`; `language/src/inbound/use_cases/project/{types,use_case}.rs` |
| Cross-file: core | a hard-coded list of project relations that need notes; project observations typed by the language concept in core storing; 17 core files in the precedent | `core/src/domain/visit/service.rs:241-243`; `core/src/domain/storing/types.rs:53-54` (`project_observations: &[ryo_language::...::EntryObservation]`); `core/src/application/project_context/service.rs`; `core/src/outbound/lint_store/adapters/postcard/tables/schemas.rs` |
| Cross-file: runtime / planner | the CrossFile / project-phase vocabulary spans runtime 17, domain 9, query_language 12, core 16, values 4 and sdk 3 files. A provider fact that is a `Project` relation reuses it, but an engine-side resolution (a fixpoint over calls) would need a new operator, which brings in change type 1's 27 match sites | `Crossing::Across` at `values/src/domain/stores/types.rs:268`; `execution/src/application/rule_compilation/identity.rs:82-105` |
| Store | new rows of a relation need no new kind. An in-engine closure (a call graph) is a new `DerivedKind`, named in 20 prod files across 4 crates (derived 2, domain 1, runtime 5, store 12) | `derived/src/domain/request/types.rs:8` |
| Core storing (what reruns) | a rule that reads across files makes any changed file a `Check` (rerun only the rules whose frontier changed); adding or changing any rule is `Whole` | `core/src/domain/storing/service.rs:54-77` |
| Benchmark crates | the precedent touched 32 files in benchmark and benchmark_contracts (request, report and context schemas carry the project observations) | e.g. `benchmark_contracts/src/domain/context/types.rs` |
| Tests in other crates | each language's conformance fixtures (`tests/fixtures/relations/<fact>/`), plus the benchmark | language-contract.md:172 |

**Totals:**
- Per-file semantic fact: **23 files in 13 crates** (ideal: 3 to 4 files in 2 crates).
- Cross-file semantic fact: **84 files in 18 crates** (52 in 16 without the benchmark crates). Ideal: about 5 files in 2 crates.
- Both totals are unchanged from 324 to 333.

**Coupling points, and the smallest change that removes each:**
1. `relation_partition` (`value.rs:225`). Declare the partition (file or project) on the relation's `relation_row!` declaration in relations, beside `class Code`, and have the domain read it. This removes the execution_domain edit.
2. One required method per relation in every language. Generate optional-capability relations into a separate capability trait, or give them a generated `absent(NotYetRead)` default, so that only an advertising language writes the method. This removes 9 language crates.
   - Caveat: this reverses the "no default body" rule (language-contract.md:5), so the owner decides it.
   - Alternative: a `Semantics` trait that only the languages implementing it implement.
3. Five registry lists in relations. Make the `relation_row!` item the single declaration, with the catalog, mod, capability enum, map and constants generated from one list. Relations goes from 6 files to 1 to 2.
4. core `visit/service.rs:241` noting list. Make "needs notes / project scope" a declared property of the relation.
5. `EntryObservation` in core storing, core project_context and 32 benchmark files. Store project observations as language-opaque digested bytes that core keeps without knowing their type. This removes the core and benchmark edits for the next project fact.
6. No cross-file provider port. Add one `Resolve`/`Link` use case to the language contract: it takes per-file facts and gives project relations, and runs as a `Project`-partition population, so the runtime is unchanged. Invalidation reuses `Decision::Check`'s frontier ("what each file shows other files").
7. If engine-side resolution is needed, change type 1's registry (operator meaning once) has to land first. Otherwise it costs about 27 match sites.

## 7. Swap one perf strategy behind its port

| Swap | 318 | 321 | 324 | 328 | 331 | 332 | 333 | Ideal |
|---|---|---|---|---|---|---|---|---|
| **Join kernel** (`JoinDriver`/`Emit`): prod files outside operators naming `equality_join`/`JoinDriver` | 4 (1 crate) | 3 (1) | 3 (2) | 3 (2) | 3 (2) | 3 (2) | 3 (2) | 0 to 1 |
| ... `Emit` sink impls | 5 | 6 | 6 | 6 | 6 | 6 | 6 | — |
| **Row representation** (proxy `Compact`): prod files naming it, plus `kinds!` | 3+1 | 6+1 | 6+1 | 6+1 | 6+1 | 6+1 | 6+1 (3 crates) | 2 (1 crate) |
| ... prod files matching `RepresentationInput::` | 8 (1) | 25 (1) | 25 (3) | 25 (3) | 25 (3) | 25 (3) | 25 (3) | 1 |
| **Relation ownership / arena** (audit row 2): files naming store concrete types, no port | 57 | 47 | 51 | 51 | 51 | 52 | 53 | 1 port + 1 adapter |
| Engine files naming `ryo_operators::` (no operators use case) | 37 | 25 | 25 | 25 | 25 | 26 | 26 | via one port |

**Isolation target:** one adapter directory behind an existing port, one registration line, and the bench picks. That is 1 to 2 files in 1 crate.

**Blocking coupling at 333:**
- **Join kernel: closest to isolated.** The port is `operators/src/domain/equality_join/types.rs:7`; the one implementation is `execution_domain/.../operator/contract/candidates.rs:111`. A streaming sink still edits the domain `replay/types.rs:13` (`Collect`), runtime `execution/output.rs:79` and `program/runtime/service/invocation/service.rs:223`.
  - Fix: move the runtime memo and buffering into the sink adapter (audit row 5).
- **Representation.** `RepresentationInput` is an enum that is destructured outside the item that owns it:
  - store `relations/identity.rs:83,109,136`;
  - store `relations/inspection/service.rs:65`;
  - store `derived/owner/acquisition/semantics/semantic_rows.rs:44`;
  - runtime `axis_resources/mod.rs:51`.

  Fix: move spans, identity, sameness and rows-of onto `RepresentationsItem`, so that store and runtime call the item and never match the enum. The swap is then item plus `kinds!` line: 2 files.
- **Relation ownership: not swappable.** `owner: Rc<RelationOwner>` at `execution_runtime/.../program/runtime/service.rs:142`, and 42 inherent store methods are called from 28 runtime files.
  - Fix: a `RelationsPort` / `WorkspacePort` in runtime's outbound, implemented by store (audit row 2).

## 8. Replace the evaluator (everything behind `FileExecution` / `ProjectRootExecution` / `run_file`): headline

**Method.**
- The evaluator's home is `crates/execution_runtime/` from 328 on. At 318 to 324 it is `crates/execution/src/application/file_dag/program/`, where `file_session`, `project_phase` and `request_execution` lived.
- The vocabulary counted is every `pub` struct, enum, trait or type declared in a home production file and declared nowhere else in the tree. That is 69 names at 333, with `Demand` dropped as a word-level false positive. Files that name `ryo_execution_runtime` directly are also counted.
- The count is the files outside the home that name any of them.
- "Vocabulary" excludes files whose only hits are the runtime's outbound ports (`DigestPort`, `DigestBytes`, `FactsPort`, `OperatorsPort`, `FactsError`), which composition wires.
- The 318 home is narrower than the later runtime crate (22 names against 69), so 318 undercounts. From 328 on, the rows compare like with like.

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 | Ideal |
|---|---|---|---|---|---|---|---|---|
| **Prod files outside the evaluator naming its internals** | 11 | 29 | 29 | 42 | 42 | 42 | **42** | 1 to 2 (the seam's adapter) |
| ... crates | 2 | 4 | 4 | 5 | 5 | 5 | **5** | 1 |
| ... of which name evaluator vocabulary, not only ports | 11 | 29 | 29 | 31 | 30 | 30 | **30** | 0 |
| **Test files outside the evaluator naming its internals** | 53 | 79 | 77 | 136 | 138 | 137 | **139** | 0 |
| ... crates | 2 | 3 | 4 | 5 | 4 | 4 | 4 | 0 |
| **Total files a rewrite must touch outside the evaluator** | 64 | 108 | 106 | 178 | 180 | 179 | **181** | 1 to 2 |

**Where the 42 production files are at 333:**
- core 25:
  - `application/cross_file/` 8 files, naming `PhaseOccurrence`, `ProjectBatch`, `PublicationSlot`, `RootOutcome`, `FrontierDemand`, `ProjectRootExecution`;
  - `application/file_analysis/` 10 files, naming `FileRunInput`, `EmittedRoot`, `CaptureContext`, `FileEnd`, `PreparedExecution`, `FactsWorker`;
  - `inbound/use_cases/lint/` 4 files;
  - `application/{engine,storing,project_context}`.
- execution 8: `inbound/use_cases/execution/interface.rs` alone has 43 `pub use ryo_execution_runtime` lines (audit row 6), plus `prepare`, `compile_rules` and `rule_compilation`.
- sdk 5 and cli 3: composition, schemas, errors.
- operators 1: `inbound/use_cases/operators/interface.rs` names `OperatorsPort`.

**Tests (139):**
- execution 94 (`crates/execution/tests`: `project_phase/`, `shared_walk/`, `submission/witness/mod.rs:138`, which reimplements `run_file`);
- core 31;
- cli 13;
- sdk 1.

**Not counted:** execution_store's roughly 24 files whose API is shaped to this evaluator. Runtime calls 42 concrete `RelationOwner`/`InvocationWorkspace` methods (change type 7). A fused or lazy evaluator would very likely reshape them as well.

**Verdict: worse.**
- Since 324 the files a rewrite must touch outside the evaluator rose from 106 to 181. Production went 29 → 42 and tests went 77 → 139.
- The 328 extraction of `execution_runtime` exposed the evaluator's internals as a crate API, instead of hiding them behind the seam. Core's `cross_file` orchestration and `file_analysis` drive phase occurrences and file sessions directly, so the outer seam is not the real seam.
- From 328 to 333 the figure is flat. No cycle reduced it.

**Isolation target:** the rewrite touches `execution_runtime` plus one adapter at the seam. Core, sdk and cli name only `ExecutionUseCase` and `PrepareUseCase` and opaque handles.

**Coupling points, and the smallest change for each:**
1. `execution/src/inbound/use_cases/execution/interface.rs`, 43 runtime re-exports. Define the seam's vocabulary in execution as opaque handles and value types, and make runtime types `pub(crate)` (audit row 6). This removes most of core's 25 files.
2. core `application/cross_file/` (8 files) runs the project phase by hand: occurrences, batches, publication slots. Move that orchestration behind `ProjectRootExecution`, so that core hands over the request and receives outcomes.
3. core `application/file_analysis/evaluation/service.rs` builds `FileRunInput`, `EmittedRoot` and `CaptureContext`. Give `FileExecution` a request/outcome pair that execution owns.
4. Runtime outbound ports (`DigestPort`, `FactsPort`, `OperatorsPort`) are wired in sdk/cli/core composition. Keep them, but declare them on execution's seam, not runtime's, so that a new evaluator implements the same ports.
5. Tests: move `crates/execution/tests/{project_phase,shared_walk,submission}` assertions into runtime and domain (audit row 9), and keep execution's tests on the two use cases. That removes about 94 test files from the blast radius.

---

## Per-checkpoint structure

### Production lines per crate

| Crate | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| execution | 56,929 | 49,030 | 18,801 | 2,334 | 2,418 | 2,426 | 2,431 |
| execution_domain | — | — | 21,229 | 21,285 | 20,998 | 21,047 | 22,059 |
| execution_runtime | — | — | — | 16,621 | 16,984 | 17,302 | 18,086 |
| execution_store | — | — | 10,393 | 10,393 | 10,394 | 10,484 | 10,484 |
| **execution total (4 crates)** | 56,929 | 49,030 | 50,423 | 50,633 | 50,794 | 51,259 | **53,060** |
| operators | 2,226 | 5,238 | 5,258 | 5,268 | 5,266 | 5,280 | 5,280 |
| core | 19,182 | 19,262 | 19,277 | 19,303 | 19,199 | 19,442 | 19,442 |
| query_language | 12,885 | 12,863 | 12,943 | 12,943 | 14,382 | 14,558 | 14,558 |
| relations | 7,900 | 7,966 | 7,986 | 7,986 | 7,986 | 7,995 | 7,995 |
| values | 6,118 | 6,249 | 6,278 | 6,278 | 6,648 | 6,765 | 6,765 |
| sdk | 5,968 | 6,006 | 6,032 | 6,034 | 6,034 | 6,040 | 6,040 |
| language | 5,787 | 5,833 | 5,896 | 5,896 | 5,896 | 5,936 | 5,936 |
| cli | 3,502 | 3,524 | 3,544 | 3,545 | 3,545 | 3,542 | 3,542 |
| derived | 2,522 | 2,964 | 2,986 | 2,986 | 2,986 | 3,006 | 3,006 |
| matching | 1,117 | 1,117 | 1,137 | 1,137 | 1,137 | 1,137 | 1,137 |
| **Engine total** | 124,136 | 120,052 | 121,760 | 122,009 | 123,873 | 124,960 | **126,761** |
| benchmark + contracts | 20,654 | 25,663 | 25,706 | 25,729 | 25,729 | 27,536 | 27,536 |
| python | 24,845 | 24,861 | 24,961 | 24,961 | 24,961 | 25,155 | 25,155 |
| all 10 languages | 76,143 | 76,159 | 76,439 | 76,439 | 76,734 | 76,966 | 77,019 |

### Structure metrics

| Metric | 318 | 321 | 324 | 328 | 331 | 332 | 333 |
|---|---|---|---|---|---|---|---|
| Whole-tree TM findings / suppressed (records) | 3,676 / 0 | 34 / 6 (24 after the lint step) | 18 / 6 | 18 / 6 | 18 / 6 (36 at `340808449f`: tracker ids) | 2 / 13 (tm 0.1.2; 1,467 under 0.1.1) | not recorded |
| Domain-concept cycle findings (records) | n/r (26 at 320) | 24 | 18 | 18 | 18 | **0** | not recorded (no domain-cycle change merged) |
| File-level ownership cycles (records) | 0 | 1 → 0 | 0 | 0 | 0 | 0 | not recorded |
| `pub(in` (engine prod) | 809 | 915 | 226 | 227 | 230 | 231 | 234 |
| `pub(crate)` (engine prod) | 2,790 | 2,857 | 2,780 | 2,773 | 2,848 | 2,890 | 2,931 |
| `dyn` uses / files (engine prod) | 667 / 251 | 841 / 270 | 854 / 276 | 850 / 276 | 848 / 278 | 870 / 281 | 876 / 284 |
| Largest concept directory (prod lines) | execution `application/file_dag` 28,356 | 25,416 | execution 15,374 + store 10,109 | runtime 15,399 + store 10,109 | runtime 15,762 + store 10,110 | runtime 16,085 + store 10,196 | runtime **16,869** + store 10,196 |
| Largest `domain/<concept>` | relations 7,080 | relations 6,756 | 6,756 | 6,756 | 6,756 | 6,763 | 6,763 (operator 3,464; executable_program 3,221, up 791 at 333) |

The 324 drop in `pub(in` (915 → 226) is mostly visibility rewritten across the new crate boundaries (`pub(in crate::…)` became `pub`). It is not narrower surfaces.

---

## Verdict

0. **Replacing the evaluator (headline) is not a focused change today, and it got worse.**
   - Outside `execution_runtime`, the rewrite must touch 42 production files in 5 crates and 139 test files: 181 in all at 333. It was 106 at 324.
   - The cause: the 328 runtime extraction published the evaluator's internals as a crate API. execution re-exports 43 of them, and core's `cross_file` and `file_analysis` drive phases and file sessions directly.
   - Fixes 1 to 5 in change type 8 (seal the seam vocabulary, move cross-file orchestration behind `ProjectRootExecution`, move the tests to their owners) would bring it to the runtime plus one adapter.
1. **Semantic analysis is not isolated today, and has not improved since 318.**
   - A per-file Python fact costs 23 files in 13 crates. A cross-file fact (the `ProjectNamespaceMembership` precedent) costs 84 files in 18 crates.
   - The target is about 4 files in 2 crates (python and relations).
   - The causes:
     - the catalog forces a method into all 10 languages;
     - five registry lists sit in relations;
     - `relation_partition` in execution_domain;
     - core's hard-coded noting list and its `EntryObservation`-typed storing;
     - there is no cross-file provider port.
   - Coupling points 1 to 6 above fix this with no runtime change.
2. **Easier: rule assets (333 only).** The noqa rewrite took per-rule pattern copies from 806 to 0, grammars from 2 to 1, and plan nodes from 112k to 44.8k. Audit rows 3 and 10 are closed.
3. **Easier: concept cycles.** Domain-concept cycle findings went 24 → 18 → 0 at 332, and whole-tree TM fell from 3,676 to 2.
4. **Easier, once:**
   - the kernel fold (321): files naming `Operation::` went 99 → 63, and the operator kernels moved into `operators`;
   - the join kernel swap now touches only 3 files outside operators.
5. **Worse: operators.** Exhaustive `Operation` matches went 21 → 27 (321 to 333), now over 8 crates instead of 5. Adding an operator gets harder every cycle, because there is no operator registry (audit row 7).
6. **Worse: the runtime lifecycle and tests.**
   - The 324 and 328 crate splits cut the lifecycle concept across store and runtime with no port: about 61 files, 40 → 46 directories, 1 → 3 crates.
   - About 50 cross-crate tests still assert runtime internals.
   - Tests that name `ryo_execution_domain` grew 131 → 157.
7. **Not better: rule edits.** A rule edit is still 2 files, but `stored.rules != rules → Decision::Whole` (core `storing/service.rs:54`) still reruns every file against every rule. That is about 294 s on Django for one rule change.
8. **Not better: store swaps.** Swapping the row representation touches 7 files in 3 crates, because store and runtime destructure `RepresentationInput`. Swapping relation ownership touches 53 files with no port.
9. **Size.** The engine grew from 124.1k to 126.8k lines. The execution crates grew from 49.0k (321) to 53.1k (333). The largest concept directory (runtime `file_dag`) grew from 15.4k to 16.9k after its split.
10. **Overall: no, not on each cycle.** The two headline cases, evaluator replacement and semantic analysis, are the least isolated changes measured. Cleanup cycles (321, 332, 333) improved local quality: TM, cycles, assets. Structural splits (324, 328) moved files without adding ports, so change amplification for the owner's two named cases stayed flat (semantic analysis, store swaps) or grew (operators, lifecycle).
    - The next gains need ports and registries, not more crate splits: the relation-declared partition and capabilities, an operator registry, `RelationsPort`, and methods on `RepresentationsItem`.
