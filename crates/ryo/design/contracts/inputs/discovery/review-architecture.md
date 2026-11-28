<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a924d4b1d0cce28f9; copied from the session transcript -->

Verdict: eleven blocking problems. I could not check the three plan-mode answers (PLAN l.20-21) because they are not yet in the record.

Shorthand: code paths are in `/workspaces/idiomatic-public-benchmark` at T=2a207464c with `crates/ryo/` omitted, or at O=f0219b3b3 where marked. PLAN = `/home/vscode/.claude/plans/squishy-sniffing-fountain.md`; LOG = `/workspaces/ryo-poc-evidence/next/build/ports/stage2/CRATES-AND-CONTRACTS.md`; STATE = `/workspaces/ryo-poc-evidence/next/engine/STATE.md`; MEM = `/home/vscode/.claude/projects/-workspaces-idiomatic/memory/`.

## Blocking

- **B1. `domain/plan` as written is a Cargo cycle, and PLAN l.93 is false.** `values::plans` is imported by 16 ryoql modules and nothing else, and `plan_parts` by 19, yet l.132 sends both to `plan`; `plan_lowering` imports `values::{program,query_syntax,query_tokens}`, which go to ryoql. The lint path parses and then lowers (`crates/executor/src/lower.rs:4-9`) and never runs ryoql's checks. There are three plan types (`values/src/domain/plans/types.rs` `PlanStep`, `FinishPlan`, `crates/executor/src/finish.rs:490`) and two planners; the third reader wrote "Owner decides" and the plan drops it. Change: send `plans`, `plan_orders`, `written_rows`, `lineage` to ryoql; send `plan_parts`, the `*Arguments` types and `query_operators` to `operators` (LOG l.136); put "two planners, three plan types" on the pass-0 page; reword l.93.
- **B2. Moving typecheck to `inbound/validate` breaks two of the plan's own rules.** `TypecheckUseCase::check` (`crates/query_language/src/inbound/use_cases/typecheck/interface.rs:9`) is "query text -> checked query", and `query.run` calls it before lint (`crates/sdk/src/resources/query/run/schemas.rs:17-26`). So either lint imports validate, or the edge sequences two features. It also leaves `ValidateUseCaseImpl` holding `Arc<dyn TypecheckUseCase>` in one crate, against "no steps, no internal use case" (MEM `hex-inbound-outbound-direction.md`). Change: `check` stays in ryoql as its contract; only `typecheck_kinds` and `keeper.rs` move; the trait split goes to the owner.
- **B3. Pass 1 rebuilds the pattern module the owner struck down.** LOG l.83: "no pattern helper survives as a crate OR as a module". After pass 1 `operators` holds names, description texts, a regex trait and a text-glob trait, and no operator; the executor compiles its own regex (`crates/executor/src/finish.rs:390`). Each operator is then spread over four crates, against "Never put the definition in one crate or directory and its implementations in others" (MEM `colocate-by-domain.md`). Change: regex and text-glob stay with their users until C2; C2's stated target is one directory per operator holding name, argument type, schema, kernel, screening summary, description and cases.
- **B4. "Restored from build 133" is not honest for C2 and C6.**
  - C2: O's signature is `run(&self, plan: &PhysicalPlan, node: PlanNodeId, buffers: &mut FileBuffers, inputs: &dyn PlanInputs, resources: &mut dyn Resources) -> Result<(), PortableOperatorFault>` (O `crates/operators/src/inbound/use_cases/run/interface.rs:32`), not `run(step, inputs, out)`.
  - Case tables: 50 of the 132 files sit under `domain/physical_plan` internals, and the run cases name `values::plans::Operation` and the row-major `values::rows::Table`.
  - C6: O's `scan(programs, file, scratch, found: &mut dyn FoundRows)` is one file with a call per found row, against "cross a port once per file" (MEM `architecture-is-for-scanning.md`).
  - Decisions D2', D3 and D4 are still OPEN (LOG l.31-32, l.46).
  - Change: call these new contracts; bring each signature diff to the owner before its cut ("bring the owner that one contract change"); state how many cases port as rows.
- **B5. There is no ledger of the build-133 contracts.** STATE 20:47: "For each crate of the agreed tree: which contract and tests come from the old crates, which code comes from the trunk, what is left over." Section 3 has no such column and §8.5 guards only T. My grep finds 63 `pub trait`s at O (the fourth reader counts 80); 46 have no line in the plan, including `PlanUseCase`, `PlanInputs`, `Resources`, `FoundRows`, `PlanCodecPort`, `LintStorePort`, `ReadStore`, `OpenUseCase`, all 15 of `execution_runtime` and all 6 of `derived`. Change: add the ledger to the pass-0 page (restored in which cut, replaced by what, or not restored with the owner's yes) and extend §8.5 to O.
- **B6. `application/execute` is still about 27,000 lines and six things after pass 3.** That is my estimate from file sizes and the first reader's ranges. No cut touches screening (7,158 lines), the session driver and admission (about 6,300, which LOG l.34 gives to lint: "admits files, drives the engine"), the project phase (2,974), plan loading, request types or bench replay. C6 contradicts itself: "lint calls only these" needs the driver in lint. PLAN l.235 then fails the owner's test (MEM `engine-small-enough-to-tune-in-one-pass.md`). Change: add cuts for driver to lint, screening, plan loading to `plan`, and one linked-language port; or withdraw l.235.
- **B7. C1 has no home crate, no signature and no sign-off; `relations <- operators` contradicts the owner.** "why do operators depend on rewlations idgi" (LOG l.108); LOG l.115 already says "`operators` states the table interface it needs". The spike and the three up-front contracts (STATE 20:45) are dropped silently. C7 names two of the six layout names the third reader lists, and is silent on `node_table` (1,986 lines). Change: state the table contract as `operators`' own port; get the table, operators and execute pages signed before C2 and C7; say whether the spike is kept.
- **B8. Pass 1's direction gate cannot pass as written.** All nine languages depend on the SDK until pass 2 (`languages/python/Cargo.toml:34`), but l.102 puts `[sdk]` above them; the command line depends on the SDK (`cli/Cargo.toml:22`) while §8.4 requires `edges-apart`. `layer-order` only forbids upward imports, so nothing enforces the edges row. Edges must also import `domain/ryoql` for its factories (`crates/sdk/src/resources/query/validate/composition.rs:40-41`). Change: give a layer list per pass, add an edge rule, and allow or move the ryoql factories.
- **B9. Pass 2 moves composition off the edge.** "the edge owned by the sdk composes" (MEM `sdk-is-an-edge.md`); "Only edges call factories" (MEM `colocate-by-domain.md`). The 2,000 lines include `composition.rs:35` (rayon), `rows.rs:33` (sysinfo) and `walk.rs:38` (ignore). Also `LintUseCase::lint(LintRequest) -> Result<..>` (`crates/core/src/inbound/use_cases/lint/interface.rs:7`) is single-request, while the live `Linter::lint(Vec<LintRequest>) -> Vec<Result<..>>` (`crates/sdk/src/resources/linter/lint/entry.rs:31`) is a batch. Change: those files stay in the SDK and are repeated in the command line; the batch signature goes to the owner; delete "each use case wires for itself" (l.90).
- **B10. Two hollow crates at build 140, and three traits with no home.** `outbound/store` would hold only "how much memory the machine has to spare" (`crates/core/src/outbound/memory/interface.rs:7`), which LOG l.34 placed in lint. `inbound/lint` would hold an unimplemented trait plus a type family that pass 2 deletes. `RowSinkPort` has no pass-1 home (l.154 lists four destinations for five ports). `values::files::Matching` and `values::words::WordOutput` appear nowhere. Change: create `store` at C1; `MemoryPort` and `RowSinkPort` go to lint; name the two traits.
- **B11. The placement rule (l.159) sorts by import graph.** It makes `relations` the new `values` (names, plus `RenderContext`, which is the render-syntax operator's argument) and parks finding places and glob types in `language`. "never park a leftover in the nearest crate" (MEM `colocate-by-domain.md`). Change: delete the rule; every item gets an owner by concept; items with several users go to the owner page.

## One-thing and one-contract test

| Crate | After pass 1 | After pass 3 | One contract? |
|---|---|---|---|
| relations | catalog + shapes and storage hints + names + `RenderContext` | catalog + two leftovers | yes |
| operators | names + descriptions + regex trait + glob trait | + kernels; arguments in `plan`, syntax tables in ryoql | pass 1 no; pass 3 only if schema and screening join `run` |
| ryoql | parser + checks + a second planner (about 5,200 lines) | same | none: `SyntaxPort` is a port it consumes |
| plan | lowering + plan types + the kind check | same | none: free functions |
| execute | everything | six things (B6) | pass 1 none (`Batch`, 30 public fields); pass 3 partial |
| language | provider traits + `LanguagePort` + node table layout + finding places + file and glob types | + author surface, macros, re-exports | two, facing opposite ways; `Source` is a third |
| store | a memory probe | tables + pool + budget | pass 1 none |
| corpus | `CorpusPort` + path-glob trait + `MatchingPort` | same | three traits for one job |
| lint | unimplemented trait + doomed types | session + types + `RowSinkPort` | pass 1 none |
| validate | `ValidateUseCase` wrapping `TypecheckUseCase` | same | two, one inside the other |

The scheduler, describe, cache (once D3 is answered), the SDK and the command line pass.

## Non-blocking

- **N1.** `execute` in `application/` contradicts the recorded tree (LOG l.423-428); record the owner's sentence first. The folder fits the nu-hex skill, but `outbound` does not: the skill says outbound imports no other layer, and the plan lets it import domain. `execute` is also not nu-hex inside (flat `src`; only `crate-executor` at `tastemaker.toml:105`), so l.79 is false for it.
- **N2.** 801 of 3,642 `api.txt` lines are the author surface (`Cargo.toml:12-19`: "versioned and semver-checked with the SDK"). Pass 2 moves it out with no listing or semver step for `outbound/language`.
- **N3.** "languages import `outbound/language` only" forces a 56-line relations re-export hub into `language`; LOG l.464 allows languages to import domain.
- **N4.** Only C2 names a contract suite. MEM `engine-contracts-engine-owned.md` requires a language-free test at each seam; O's `boundary_contract` (171 files) and `core/tests/fixtures/operators` (30) are unassigned.
- **N5.** Not in the tree and not mentioned: rule packs and their encoding, observations and hosted inputs, text interning, the fake language, and the ten `fixtures` features once `values` and `core` dissolve.
- **N6.** C7 must move all nine providers in one landing ("a single contract, never two", MEM `one-language-contract-never-two.md`).
- **N7.** At least 16 Tastemaker definitions carry such globs, not four; `no-serde-in-domain/0.1.0:11` hard-codes `crates/sdk/src`. `language-depends-on-ryo-only/0.2.0` needs a new version, not a rewrite.
- **N8.** Seven carved-out crates merge at build 140 without the one-page contract each (STATE 20:45).
- **N9.** The kind check was recorded in ryoql (LOG l.99); its move to `plan` is not told to the owner.

### Critical Files for Implementation
- `/home/vscode/.claude/plans/squishy-sniffing-fountain.md`
- `/workspaces/ryo-poc-evidence/next/build/ports/stage2/CRATES-AND-CONTRACTS.md`
- `crates/ryo/crates/values/src/domain/plans/types.rs` (T, with `crates/ryo/crates/executor/src/finish/plan_load.rs`)
- `crates/ryo/crates/operators/src/inbound/use_cases/run/interface.rs` (O)
- `crates/ryo/tastemaker.toml` (T, lines 35-36 and 105)
