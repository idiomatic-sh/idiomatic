# Annotation evaluation positions

The helper accepts real expression occurrences and returns `id,file,evaluation` for their directly containing annotation context:

- `eager`: evaluated when its owning definition/statement executes, through Python 3.13 without the annotations future.
- `postponed`: a stored annotation string under the validated module future feature.
- `deferred`: a stored native lazy annotation from Python 3.14 onward, without that future feature.
- `unevaluated`: a function-local variable annotation, or a non-simple variable annotation under the future/native lazy modes.

Function signatures and module/class variable annotations are eligible. Defaults, decorators, targets, class bases, type-alias values and type-parameter bounds/defaults are not ordinary annotation inputs. Nested lambda/generator bodies and quoted-type embeddings have their own evaluation/typing contexts; they do not inherit the enclosing expression's direct timing. A generator's first iterable remains part of its enclosing expression's evaluation.

A simple annotated assignment has an unparenthesized Name target. Existing real statement/target byte starts distinguish `name: T` from `(name): T`; no source positions are fabricated. Scope ancestry distinguishes function-local annotations from module/class annotations, including classes nested in functions.

The classification uses the canonical `compiler-feature` flag and configured `language-target`. Missing capability is unknown and cannot imply eager evaluation. It is independent of `.py`/`.pyi` filenames; consumers may implement a separate, explicit stub typing policy. This helper is not a linter worklist or namespace snapshot.

Independent `compile(..., dont_inherit=True)` and runtime observations for CPython 3.10, 3.13, 3.14 and 3.15 are retained in `tests/fixtures/compiler-features`. The [Python 3.14 assignment reference](https://docs.python.org/3.14/reference/simple_stmts.html#annotated-assignment-statements) documents native lazy annotations, non-simple targets and unevaluated function-local annotations.
