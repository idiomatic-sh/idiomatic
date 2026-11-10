# Leak review: checkpoint 334, post-freeze delta

- **Reviewed:** `a33f361f7a7f1421358fe006a7b88fa5f9456ff4` (claude/checkpoint-334).
- **Base:** `333641a62a01e87ce8bab84f94a43a710f813b9a`. That sha's leak pass failed check 2 only; every other check carries forward.
- **Delta:** `b6aff49ae3` (sdk_api.py reads the internal crates from the SDK manifest), `c00074ffe6` (run.py writes one line per check), and the record commits `a01a31de23` and `a33f361f7a`.

The reviewed tree stayed at the same HEAD before and after. Its working-tree changes come from the concurrent contract review, not from this review. The probes ran in a `git archive` copy under scratch, which was deleted afterwards.

## Delta hygiene

- **No Rust.** `git diff --name-only 333641a62a..a33f361f7a` lists no `.rs` or `Cargo.toml` file.
- **No tracker ids.** The only `ryo-` tokens added are package names (`idiomatic-sh-ryo-core` and the like).
- **Fewer machine paths.** The record's commands now use `{target}` and `{temp}` where they used an absolute target path and an absolute temporary path.
- **Advisory.** `validation.json` still carries `raw_identity` as an absolute temporary path (since named under `{temp}`). This is the existing raw-log field that `run.py:1089` writes and `run.py:282,517` reads. The delta only bumped its number (it held `...333` before), so it is not new in this delta.

## Check 1: proof where the contract lives (PASS, carried forward)

The delta touches no proof.

## Check 2: every enforcement claim fires (FAIL)

**The four direct crates are fixed.** `sdk_api.py:65` `internal_crates()` reads the SDK manifest's path dependencies. On `crates/ryo/crates/sdk/Cargo.toml` it returns nine crates. I re-ran the `leaks()` probe from the prior pass: one public SDK function returning a type of the named crate.

| Crate that defines the returned type | Leaks reported |
| --- | --- |
| `ryo_core` (control) | 1 |
| `ryo_execution` | 1 |
| `ryo_execution_runtime` | 1 |
| `ryo_operators` | 1 |
| `ryo_matching` | 1 |
| `idiomatic_sh_ryo_execution`, `idiomatic_sh_ryo_matching` | 1 each |
| `ryo_relations`, `ryo_language`, `ryo_values`, `ryo_query_language` | 1 each |
| `serde`, `rayon`, `thiserror` | 0 |
| `ryo_execution_domain` | **0** |
| `ryo_execution_store` | **0** |
| `ryo_derived` | **0** |

**The same class is still open for transitive internal crates.**

- `internal_crates()` reads only the SDK's own `[dependencies]`.
- `ryo_execution` forwards types defined in `ryo_execution_domain` (`execution/src/inbound/use_cases/execution/interface.rs:47-56`: `Covered`, `Partials`, `RuleIndex`, `Arena`, `ObservedFrontierRoot` and more).
- `crates/ryo/tastemaker.toml:17` allows the SDK to name exactly those, through `ryo_execution`.
- rustdoc records the defining crate, `ryo_execution_domain`. That crate is not in the set, so `leaks()` (`sdk_api.py:134`) stays silent.

So a contributor could plausibly write `pub fn covered() -> ryo_execution::...::interface::Covered` in the SDK, re-record `api.txt`, and the gate would pass.

- **Is it a live leak?** No. `crates/sdk/api.txt` names none of these types.
- **Why this fails here:** the prior report listed these reachable crates explicitly, and this review fails a check until every variant of its class is closed.

**Required change.** Fix the harness itself; do not add a parallel test.
- Build the internal set from the transitive closure of the SDK's workspace path dependencies. The `cargo metadata` that `check()` already loads (`sdk_api.py:174`) has the resolve graph. Alternatively, read the `internal-packages` variable at `crates/ryo/tastemaker.toml:8`.
- Extend the manifest case in `sdk_api_test.py` so that a `ryo_execution_domain` type forwarded through `ryo_execution` is refused.

**Other harness paths in the delta.**
- `sdk_api_test.py`: 18 of 18 pass in the archive copy.
- `run.py` `dumps()` (`run.py:26-36`): the change only reformats the output. Its test (`run_test.py:203-209`) asserts `json.loads(text) == v`, so no failure can turn into a pass.

**Plants carried forward** (seal, fixtures gate, `unused_must_use`, `unfulfilled_lint_expectations`): each fired once and was reverted. No Rust changed since, so these results still hold.

## Check 3: boundaries hold across crates (PASS, carried forward)

The delta has no Rust and no manifest change.

## Check 4: one implementation and one representation per concept (PASS)

- The hand-written `INTERNAL` tuple is deleted. `internal_crates()` is now the one definition the check uses.
- `sdk_api_test.py:21` `INTERNAL` is only an input to the synthetic leak cases.
- `test_the_ryo_sdk_manifest_names_the_execution_crates_internal` reads the real manifest.

## Check 5: layer direction (PASS, carried forward)

The delta has no Rust.

## Check 6: test seams stay out of production (PASS, carried forward)

The delta has no Rust.

## Items 16-18 (PASS, carried forward)

The libtest-protocol advisory is unchanged.

## Enforcement candidates

| Id | Status |
| --- | --- |
| `sdk-api-internal-crate-list-hand-written-8fa957e4` | fixed (the class stays for the TM steward to decide) |
| `home-grown-libtest-argument-protocol-e9dbc575` | open |
| `sdk-api-internal-crates-direct-dependencies-only-8edfee08` | open (new): strengthen the first candidate to the transitive closure |

VERDICT: FAIL: 2
