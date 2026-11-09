# Leak review: claude/w15-loop at b59b22c6e7, round 5

Base dcea2ee7df. Reviewed tree: the builder's worktree, HEAD b59b22c6e72b9c84f63c1ca0bcc2a8395dfddca5, clean status before and after the review.

This round re-checks the round-4 finding (census-cfg-test-angle-bracket-depth-334fc686) and spot-checks the round-1 to round-4 fixes. It is graded under the hooks README's threat model, which covers honest mistakes, not obfuscation. Plants ran in a `git archive` copy, which was deleted afterwards. Evidence: `claude-w15-loop.json` beside this file, with 10 plants and 32 enforcement candidates. 334fc686 is now fixed, and one candidate is new.

## Check 1: proof where the contract lives: PASS
No Rust source changes. The only manifest edit is crates/ryo/Cargo.toml:149-152.

## Check 2: every enforcement claim fires: FAIL
**The round-4 escape is closed (run.py:522-538).** Angle brackets count only while `header` is true, and the first `{` sets it to false. Bodies track `{}()[]`. `literal_end` (run.py:510-521) skips comments, strings, raw strings and char literals, and leaves lifetimes alone.

Each plant below sat before a method in the same impl that matches 8 Operation variants: the count is 24 without that method and 25 with it. All of these stay at 25:
- `self.seen < 4`, `n << 2` and `if n < 4 {}`
- `'{'`, `"("` and `r#"}{"#`
- a `}` in a comment
- lifetimes
- nested generics with a closure `> 1 && >= 2`
- a where clause
- `[b'{', b'(']`

The round-4 advisory on literals is closed too. run_test.py:254 has a regression case.

**New escape, a variant of the same class: an initializer counts as header (run.py:529, 531).** For a const, static or enum discriminant, the header runs up to the `;` or `,`, so a `<` or `<<` in the initializer raises the depth. The depth is still above 0 at the `;`, and the following production items count as test.
- Plant: either line below takes operation_exhaustive_matches from 25 to 24.
  ```rust
  #[cfg(test)]
  const LIMIT: usize = 1 << 10;
  ```
  The same happens with `const SMALL: bool = 3 < 4;`.
- This shape is plausible here. core/src/outbound/lint_store/adapters/memory/constants.rs:4-5 is a `#[cfg(test)]` const in a production file. core/src/domain/ordering/service.rs:138 and :153, and execution_domain/src/domain/comparing/service.rs:33, are `1 << N` consts.
- Required change: end the header at a depth-0 `=` as well as at `{`. After that, count `{}()[]` only. Add a run_test case with `#[cfg(test)] const X: usize = 1 << 10;` and `const B: bool = 3 < 4;` ahead of production code.

**Rounds 1-4 still hold.**
- .claude/ is byte-identical to 1c2a73da64.
- test_hooks passes. It still includes the cases for `flock --wait` and `--shared` (test_hooks.py:171-191) and for a token named twice (test_hooks.py:1385-1403).
- is_test_file treats `use_case_test/` as test, and `latest/` and `contest/` as production.
- A cfg(test) field still ends at its comma.
- run_test passes all 41 tests.

## Check 3: boundaries hold across crates: PASS
The repair touches only run.py and run_test.py.

## Check 4: one implementation and one representation per concept: PASS
`item_end` and `literal_end` are the only extent scanner, and `is_test_file` is the only test-file classifier.

## Check 5: layer direction: PASS
No .rs file changed in dcea2ee7df..b59b22c6e7.

## Check 6: test seams stay out of production: PASS
No feature, cfg(test) bypass or public test helper was added.

## Beads (one per class, for the PM to file)
- census cfg(test) const, static or discriminant initializer counted as header, so `<` or `<<` raises the depth: run.py:529-531 (census-cfg-test-header-initializer-angle-c993d1af). Under ryo-xzox, this shape becomes a case for the replacement query.

VERDICT: FAIL: 2
