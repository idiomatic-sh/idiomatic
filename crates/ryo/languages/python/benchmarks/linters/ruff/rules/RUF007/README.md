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

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/RUF007/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 4 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `beyond-i32`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select RUF007 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (4): `beyond-i32`, `beyond-i32-both`, `beyond-f64-integer-precision`, `beyond-i128`.

Representative exact source, `beyond-i32` (`case.py`):

```python
result = zip(xs[2147483647:], xs[2147483648:])
```

- Intended: RUF007 case.py:9–12. Pinned Ruff observed: none.
- Basis: Python accepts the exact consecutive slice starts across signed 32-bit range; the pinned reference omits the idiom at its integer conversion boundary. Primary evidence: [cpython310-observation.json](../../profiles/RUF007/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/RUF007/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
