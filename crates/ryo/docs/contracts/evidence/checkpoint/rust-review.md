# Rust gate, checkpoint 334 (final, diff-scoped): PASS

- **Reviewed:** `333641a62a01e87ce8bab84f94a43a710f813b9a` against base `40e49021520d4392d29df28ee5bd2fef739e5d37` (333 head).
- **Criteria items:** the owner table in `.agents/agents/review-criteria.md` gives this gate items 3, 6, 8, 9, 11, 12, 13, 14, 18, 19 and 21. Five map into the seven checks: 3 to items 1 and 2, 6 to item 4, 8 to item 6, 11 to item 3, and 18 to item 5. The other six are graded as their own checks: 9, 12, 13, 14, 19 and 21. Items 12, 13 and 14 gate the verdict.
- **Tree check:** HEAD was 333641a62a both at the start and at the end. `git status --porcelain` showed the same single line both times: `M crates/ryo/docs/contracts/evidence/checkpoint/identity.json`, written by the concurrent validation. The pass did not change it, and no source file changed.
- **Scope:** the code graded is the code at this sha. The validation run happening alongside this review (cargo-doc, the doctests, and validation.json's source) is the integrator's to record.

## The early FAILs (293766d1a3), each closed
| Early item | Finding | Closed by | Evidence |
| --- | --- | --- | --- |
| 1, 13 | RelationStore was unsealed, and its `fixture` method depended on a feature | 2e9505dfca | `interface.rs:16-22,110` gives `RelationStore: sealed::Sealed`. `fixture` is now an inherent `RelationOwner` constructor. A compile_fail,E0277 doctest was recorded red, then green. |
| 1 | ExecutablePermission had no example | 940a264aaa | `sdk/src/domain/file/types.rs:32-44` |
| 5 | run.py had a hand-written Rust scanner | 351706d970, cc649856d9, d1b4db5f41 | `run.py:612-620` uses the tree-sitter-rust Parser and Query. `literal_end`, `item_end` and `split_tests` are deleted. The pin is in `tools/install.py` (ffadf1b580). |
| 6, 9 | cargo_diagnostics dropped lints denied by a group or by default | ec498afe71, 63f3f9e3f7 | `cargo_diagnostics.py:45-66,88-110` reads the level from the compiler's note per package, reads default levels from the toolchain tables, and fails closed. Both red-then-green entries are in validation.json. |
| 12 | 32 production `usize::MAX` sentinels | 889764df1f, 477b650641 | The diff now adds none. Each index type has one `From` (`vertices/types.rs:8-30`, `relations/types.rs`, `plan_parts/types.rs`). gate:no_usize_sentinels was recorded red, then green. |
| advisory | The counting observer's totals accumulated across runs | c3bf3e75cd | `use_case.rs:96,173-181` holds `Option<fn() -> Arc<dyn ObserverPort>>` and builds a fresh observer per run. The test `two_counted_lints_of_one_use_case_each_report_only_their_own_rows` was red at (2, 4) and green after. |

## 1 Ownership and API shape: PASS
- The store seam is sealed (see the table above). The observer is a function pointer that each run calls, so no `Arc` is shared across runs.
- The SDK api.txt has not changed since the early gate, which accepted it. The new public SDK item has docs and an example.

## 2 Compiler over tests: PASS
- Privacy and the seal enforce the seam. The doctests only pin it.
- F3 part 2 changed five `Operation` matches to end in `_` under a reasoned `#[expect(clippy::wildcard_enum_match_arm)]`, for example `prepared/value.rs:53` and `self_join_recogniser/service.rs:82`.
  - The compiler still stops a new variant: `operator/contract/service.rs:204-257` `failure()` names every variant exhaustively.
  - The variant tests from 64f77797ef pin the grouped arms on top of that compiler stop.

## 3 Hot path: PASS
- `From<Index> for usize` compiles to a plain cast. The per-edge kernel lost its try_from-plus-sentinel branch (`adjacency/service.rs:108-117`).
- The observer is built once per run.
- The doc batch (018413103a) contains only doc comments.

## 4 One representation per concept: PASS
- `PureSelection<'a>` (`step_classes/types.rs:33`) is a borrowed view. It folds two copies of the Select/Project extraction into one function, `pure_selection`.

## 5 Standard crate over home-grown: PASS
- run.py reads Rust with tree-sitter-rust.
- `harness_case` (`samples.rs:184-244`) is about 60 lines of libtest-protocol glue, shared by three runners. Cargo, uv and ruff use no crate for this. libtest-mimic is listed as an advisory.

## 6 Tests: PASS
- The harness is repaired in place, not worked around by a parallel test.
  - The filter fails closed: a lint whose level cannot be read is forwarded, with a line on stderr.
  - `cargo_diagnostics_test.py:42-104` covers group denies, default denies, a default the toolchain does not give, a later diagnostic with no note, and a level that cannot be read.
- Every fix since the early gate names its test, and each has a red_green entry in validation.json.
- 752cc02c32 changes only commands.json, an evidence record. The later gate entries prove its effect.
- Of the 18 assert lines removed since the early gate, every one comes back reformatted by rustfmt (da77293b16). None was weakened.

## 7 Rust fit: PASS
- The diff adds no `unsafe` and no `#[allow]`.
- Every new `as` and wildcard sits under `#[expect(..., reason)]`, and `unfulfilled_lint_expectations = "deny"` holds each one.
- The `expect` calls in `samples.rs` are in a fixtures module gated by `cfg(any(test, feature = "fixtures"))`.

## 9 Cheapest enforcement first: PASS
- The loose ignore is repaired.
- Each new suppression states a real reason and is held by the deny on unfulfilled expectations.

## 12 Idiomatic Rust: PASS
- No sentinel conversions are added. The conversions follow C-CONV: one `From` impl on each index type, in that type's own types.rs.
- The remaining sentinels in core cross_file and file_analysis predate this diff, so they are outside this gate's scope.

## 13 API stability: PASS
- Turning on `fixtures` no longer adds a method to RelationStore, so the feature is additive again.
- The semver baseline is the released tag `idiomatic-sh-ryo-v0.1.2` (`Cargo.toml:16`).
- The F3 seals narrow the surface.

## 14 Documentation: PASS
- The doc batch adds 51 examples across the three execution crates, plus the ExecutablePermission example.
- The `# Errors` and `# Panics` lints are at deny.
- The 334 cargo-doc and doctest runs are being recorded by the concurrent validation.

## 19 Not a lot of code: PASS
- Production Rust changes since the early gate are small: conversions, seals, the 17-line observer fix, and examples.
- Overall the diff deletes far more than it adds.

## 21 One of N: PASS
- One `harness_case` serves core, the SDK language contract and every language's allocation runner. It is not copied into each.

## Advisories (open-source hygiene; none changes the verdict)
1. **harness_case parses libtest's arguments by hand** (`language/src/inbound/use_cases/parse/fixtures/samples.rs:190-224`). Fix: use `libtest-mimic` (it is what nextest's docs suggest for custom harnesses), or record why not, for example its single maintainer.
2. **`From<Index> for usize` loses data where usize is narrower than 32 bits** (`derived/src/domain/vertices/types.rs:8-30`, `relations/types.rs`, `values/.../plan_parts/types.rs`). Fix: put `const _: () = assert!(usize::BITS >= 32);` beside the impls, so the reason the `#[expect]` gives is checked by the compiler.
3. **The public SDK function `allocations_of_linked` changed behaviour** (`sdk/src/resources/languages/contract.rs:215`). It now reads `std::env::args` and prints libtest text to stdout. Fix: record this for language authors in the SDK changelog when one exists.
4. **run.py's `identical` proof entries** (d1b4db5f41) accept two logs with equal bytes but do not tie them to the code before and after. Fix: record the sha each log was produced at, and check those against the range.
5. **run.py parses four test runners' human-readable output with regexes** (`run.py:405-418`). Fix: have nextest and pytest write JUnit, and read that.
6. **Still open from the early gate:**
   - `rule_compilation/service.rs:128`: the `Option` slots plus a reasoned `expect` could collect into a typed refusal.
   - `Cargo.toml:105`: `missing_docs = "warn"` is never gated.

## Beads for the PM
None are required, because no item FAILs. File each advisory above as a bead, or drop it with a one-line reason.
