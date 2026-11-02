# RUF007 source candidate

This query identifies the successive-pair idiom at a proven builtin/imported
`zip` call with exactly two positional arguments. Both receivers must spell the
same direct name. The first argument may be that name or a slice; the second
must be a slice. An omitted lower bound means zero. A written lower bound must
be a direct integer literal of arbitrary precision, and a written step must be direct
integer one. Numeric spellings are decoded by the provider. Unary negative/plus
bounds, float/bool bounds or steps, dynamic starts, and other steps are excluded.
Nonnegativity follows from direct Python literal syntax; a written minus is a
separate unary node. The pinned checker's incidental i32 conversion bound is not
a semantic restriction of this candidate. Large-start discrepancy predictions
remain pending actual reference execution and independent evidence.
The diagnostic uses the actual function-expression span.

Exact arbitrary starts use the generic `integer_successor(first, second)`
predicate, meaning `second == first + 1` for integers only. It has no float
coercion and uses the shared numeric domain. The query does not implement decimal
carry arithmetic or infer adjacency from the literals that happen to occur.
Positional pairing happens only after proving arity two, using `ordered-match`;
there is no all-pair argument scan or assumption about global child ordinals.

This is an explicit idiom diagnostic policy, not an engine fact asserting that
`pairwise(name)` preserves the original call's behavior. Upper bounds are retained
as part of that policy rather than normalized using an unknown sequence length.
For example, nonzero starts, clipped bounds and short/empty sequences need care
when constructing a replacement; custom slicing can differ from iteration, and
`strict` can change exhaustion behavior. No automatic fix or equivalence claim is
emitted. Negative lower bounds are separate unary syntax and do not enter the
successor test. Fixtures exercise arbitrary/negative upper bounds, clipped and
empty slice controls, keyword `strict`, custom/unknown receivers, name mismatch,
shadowed `zip`, and explicit builtin imports.

The singleton profile targets Python 3.10 and the pinned accepted-syntax phase.
It declares only the exact existing suppression closure. Source policy audit:
[`zip_instead_of_pairwise.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/ruff/rules/zip_instead_of_pairwise.rs),
Ruff 0.16.5 commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`.
The tagged upstream fixture is included intact. Canonical runtime validation is
pending; no reference exception, admission or semantic rewrite proof is claimed.
