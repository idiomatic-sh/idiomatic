# Leak review: claude/w15-loop at 794ec68025 (delta b59b22c6e7..794ec68025)

The delta touches four files and no Rust source:
- `.agents/agents/leak-review.md:13`
- `.claude/hooks/README.md:14-16`
- `crates/ryo/docs/contracts/evidence/checkpoint/run.py:523-530`
- `crates/ryo/docs/contracts/evidence/checkpoint/run_test.py:261-267`

## Check 1: proof where the contract lives: PASS
No Rust source or manifest changed. The census reads source statically and proves no contract.

## Check 2: every enforcement claim fires: PASS
The open finding census-cfg-test-header-initializer-angle-c993d1af is closed. run.py:530 ends the header at a depth-0 `=` that is not part of `==`, `=>`, `<=`, `>=` or `!=`.

Plants were made in a git archive copy, in the real file `core/src/outbound/lint_store/adapters/memory/constants.rs`. Each was followed by a production fn naming 8 Operation variants. The control count is 25.
- Positive control: b59b22c6e7's run.py drops `#[cfg(test)] const LIMIT: usize = 1 << 10;` and `const SMALL: bool = 3 < 4;` to 24, so the escape is reproduced.
- With 794ec68025's run.py, these all stay at 25:
  - LIMIT and SMALL
  - `static S: [u8; 2] = [1 << 2, 0];`
  - a cfg(test) discriminant `A = 1 << 3,`
  - `const V: Vec<Option<u8>>`
  - a closure const with `x > 1 && x >= 2`
  - `type A<T = u8>`
  - mixed `<= != >= << ==`
  - `Iterator<Item = u8>` with a where clause
  - macro_rules `=>`
  - an impl holding an associated const
- run_test passes 42 tests. run_test.py:261 is the new regression case.
- The hooks change is README text only, so hook code is unchanged since round 4.

**Advisory under the recorded decision.** leak-review.md:13 and the hooks README:14-16 make further census-scanner edge cases advisories owned by the ryo-query replacement, unless the shape occurs in the real tree.
- An array length in the header still escapes. `#[cfg(test)] fn arr() -> [u8; 1 << 2]` and `const A: [u8; 1 << 2]` drop the count to 24.
- `git grep` at 794ec68025 finds this shape only inside string literals in test files, never in Rust code. So it is an advisory, not a FAIL.
- It is carried as census-cfg-test-header-array-length-angle-1f312aff for the replacement query.

## Check 3: boundaries hold across crates: PASS
The delta changes no visibility, allowlist, import boundary or feature.

## Check 4: one implementation and one representation per concept: PASS
item_end (run.py:522) is still the only extent scanner. The fix edits it in place.

## Check 5: layer direction: PASS
The delta changes no .rs file.

## Check 6: test seams stay out of production: PASS
The delta adds no feature, cfg(test) bypass or public test helper.

## Enforcement candidates
- All earlier entries are carried forward, and c993d1af is now `fixed`.
- New `open` advisory: census-cfg-test-header-array-length-angle-1f312aff.

VERDICT: PASS
