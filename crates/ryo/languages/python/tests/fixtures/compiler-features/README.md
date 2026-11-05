# Compiler features and annotation evaluation

These are independent Python language expectations, not replacement Ruff reference arrays. Historical competitor outputs and frozen cohort receipts remain unchanged.

`annotation-contexts.json` supplies 51 real-occurrence checks through the local SDK adapter. Every case must produce an INPUT finding for its existing syntax node before mode findings are compared. The checks distinguish the validated module future flag from eager, postponed-string, native deferred and unevaluated annotation positions. They cover module/class/signature/local contexts, parenthesized targets, defaults/decorators, lambda/generator boundaries, aliases/type parameters, constants with no binding-read witness, source suffixes and Unicode byte positions.

The canonical frontend alone determines whether an absolute initial module future directive is legal. Producer tests cover the optional first docstring, multiple directives, aliases, relative/late/nested/lazy/malformed imports and a later invalid directive. A missing optional capability remains unknown; it never emits a disabled feature row. No statement/handler snapshot or linter traversal order is inferred by the query.

`probe.py` observes effects at definition execution; `access_probe.py` also reads actual `__annotations__`. The retained CPython 3.10.19, 3.13.5, 3.14.0 and 3.15.0rc2 results were run in isolated interpreter mode and explicitly use `compile(..., dont_inherit=True)`. Every recorded case completed without an exception. `provenance.json` binds source scripts, executable identities, versions and observation bytes.

The [Python 3.14 assignment reference](https://docs.python.org/3.14/reference/simple_stmts.html#annotated-assignment-statements) defines native lazy annotations for simple module/class targets and signatures, unevaluated non-simple targets, and unevaluated function-local annotations. The [future statement reference](https://docs.python.org/3.14/reference/simple_stmts.html#future-statements) defines the legal absolute module-prefix directive. Relative-future implementation differences are not asserted as universal interpreter defects; the documented compiler directive contract governs the producer.

This fixture is a regression input, not a completed test receipt. Integration must compile the new optional relation and run `compiler_features_and_annotation_contexts_follow_the_language_contract`, producer reader tests, unsupported-capability coverage and the language/contract TM proofs before claiming verification.
