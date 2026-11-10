# Leak review: checkpoint 334, checkpoint-level pass

- **Reviewed:** `333641a62a01e87ce8bab84f94a43a710f813b9a` (claude/checkpoint-334).
- **Base:** `40e49021520d4392d29df28ee5bd2fef739e5d37` (333).
- **Scope:** 19 branch merges, 744 Rust files.
- **Focus:** leaks that appear only when the branches combine. Every merged branch already has its own leak PASS.

The reviewed tree stayed at the same HEAD before and after. Its working-tree changes come from the concurrent Rust review (`rust-review*.json` and `.md`, `identity.json`), not from this review. The probes ran in a `git archive` copy with their own check target. The hook allows only package-scoped `cargo check` outside an integration tree, so rustdoc, clippy and `cargo tree` were not run. The copy and its target were deleted afterwards.

## Check 1: proof where the contract lives (PASS)

- The Ruff benchmark is now its own unpublished package (`languages/python/benchmarks/linters/ruff/Cargo.toml`).
- `8221fa41c9` moved the engine, SDK and provider proofs it held to their owning crates.
- `engine_test/authored_queries_test.rs:5` validates only the benchmark's own queries, through the public SDK.
- The port-suite seam suite lives in `crates/execution/tests`.

## Check 2: every enforcement claim fires (FAIL)

**Finding: the sdk-api leak gate does not cover four of the SDK's nine internal dependencies.**

`ci/scripts/checks/sdk_api.py:30` hard-codes this tuple:

```
INTERNAL = ("ryo_core", "ryo_language", "ryo_relations", "ryo_values", "ryo_query_language")
```

- The SDK also depends directly on `ryo_execution`, `ryo_execution_runtime`, `ryo_operators` and `ryo_matching` (`crates/ryo/crates/sdk/Cargo.toml:18-23`).
- Through those crates it can reach types defined in `ryo_execution_domain`, `ryo_execution_store` and `ryo_derived`.
- `leaks()` (`sdk_api.py:128`) reports a type only when its crate is in `INTERNAL`.

**Probe.** I loaded `sdk_api.py` and called `leaks()` on a minimal rustdoc document: one public SDK function returning a foreign type.

| Crate that defines the returned type | Leaks reported |
| --- | --- |
| `ryo_core` | 1 |
| `ryo_execution_runtime` | 0 |
| `ryo_execution` | 0 |
| `ryo_operators` | 0 |
| `ryo_matching` | 0 |

So once the listing is re-recorded, an SDK that hands out an execution, operator or matching type passes the gate.

- **Is it a live leak?** No. `crates/sdk/api.txt` names no type from those crates today.
- **Why it counts here:** the hole predates 334, but the script is in this diff's closure (audit-f3 moved it to `--all-features`), and check 2 covers every runner in the closure.

**Required change.** Derive `INTERNAL` from one definition: the `internal-packages` variable in `crates/ryo/tastemaker.toml:8`, or the SDK manifest's path dependencies. Add an `sdk_api_test.py` case where an execution-crate type in a public SDK signature is refused. Fix the harness itself; do not add a parallel test.

**Plants that fired, each reverted:**

| Claim | Plant | Count |
| --- | --- | --- |
| RelationStore seal | `execution_runtime` implementing `sealed::Sealed` | 1 (E0603) |
| Fixtures gate | `execution_runtime` production calling `RelationOwner::fixture` | 1 (E0599) |
| Fixtures gate | CLI library (default features) re-exporting core's lint fixtures | 1 (E0432) |
| `unused_must_use` deny | dropped `Result` in `crates/sdk/src/lib.rs` | 1 |
| `unfulfilled_lint_expectations` deny | `#[expect(dead_code)]` on a pub fn | 1 |

**Merge resolutions.** All seven non-trivial resolutions were read; none dropped a guard:
- `99482c61f8` (two runtime files): kept the L-span `validate_spans` call and the sealed `publish` route.
- The rest touch `run.py`, `run_test.py`, the agent definitions, `commands.json` and `prepared.json`.

## Check 3: boundaries hold across crates (PASS)

- **Manifest scan.** No normal or build dependency enables `fixtures`, across every tracked `Cargo.toml`.
- **Plants:**
  - Implementing the sealed trait from another crate fails.
  - Calling the gated `RelationOwner::fixture` (`relations.rs:167`) from `execution_runtime` production code fails.
  - The CLI production build cannot reach a fixtures module.
- **Feature combinations.** The SDK checks under `--no-default-features` and under `--all-features` (with runtime, store and core). The Ruff package checks with `--all-targets`.
- **SDK surface.** `api.txt` reaches only `ryo_language` and `ryo_relations` types, and each one sits under `ryo::languages` or is re-exported there. Checked: `ByteSlice`, `Reading`, `SampleText`, `Oracle`, `Shape`, `SourcePath`, `ReadItem`, `CellKind`.

## Check 4: one implementation and one representation per concept (PASS)

- The port-suite merge resolved `preparation/service.rs` to `program.matchers()`. The one `PreparedMatchers` construction is `executable_program/service.rs:48`, and `from_operations` is `pub(crate)`.
- `harness_case` has one definition (`language .../parse/fixtures/samples.rs:184`). Core, language and the SDK all call it.
- `Cargo.lock` gains no third-party crate.

## Check 5: layer direction (PASS)

- **Crate direction.** `execution_domain` → `execution_store` → `execution_runtime` → `execution` → `core` → `sdk`. No normal dependency points back.
- **Merge resolutions.** They reach the store only through the sealed `RelationStore`.
- **SDK wiring.** `LinterLintEngine` stays `pub(crate)` (`sdk .../linter/lint/types.rs:159`).

## Check 6: test seams stay out of production (PASS)

- **FileSession counters.** `fixture_readiness_counts`, `fixture_walk_counts` and `fixture_counts` sit inside the `#[cfg(any(test, feature = "fixtures"))]` impl (`file_session/service.rs:126`).
- **Other gated items:**
  - ProjectPhase's matching methods (`project_phase/service.rs:229,245`).
  - `RelationOwner::fixture` and `stable` (`relations.rs:167,175`).
  - `observer::fixtures`.
- **Seals.** The audit-f2, audit-f3 and audit-f3-part2 merges sealed 116 items to `pub(crate)` or `pub(super)`. None is plain `pub` again at the landed sha, so no later branch reopened a seal.
- **Advisory (before 334).** `LintUseCaseImpl::counting` (core `lint/use_case.rs:173`) is a `pub` builder whose only callers are fixtures and tests.

## Item 16: dependencies and supply chain (PASS)

- The lock diff adds one package, the internal Ruff benchmark package, and no third-party crate.
- The Ruff package's crates are all dev-dependencies.
- The jemalloc and mimalloc allocators stay target-gated dev-dependencies.

## Item 17: portable and reproducible (PASS)

- `tree-sitter` and `tree-sitter-rust` are pinned in `tools/install.py:129`.
- The new `located_index` HashMap (`presence/types.rs:199`) is a lookup and is never iterated into output.

## Item 18: use what everyone uses (PASS, one advisory)

- `run.py`'s hand-written Rust scanner was replaced by tree-sitter-rust.
- **Advisory.** Three gate-fix branches together grew `harness_case` (`samples.rs:184-224`) into an emulation of libtest's command line: `--list`, `--ignored`, `--exact`, `--skip` and the value options.
  - `libtest-mimic` is the purpose-built crate, but it has one maintainer and the glue is small, so this is not a FAIL.
  - No path in it turns a failure into a pass: a panic resumes after the report, so the run exits 101.
  - Handed to the TM steward as a candidate.

## Enforcement candidates

| Id | Status |
| --- | --- |
| `sdk-api-internal-crate-list-hand-written-8fa957e4` | open |
| `home-grown-libtest-argument-protocol-e9dbc575` | open |

VERDICT: FAIL: 2
