# Module compiler feature

`future-annotation-context(expressions=<id: node, file: file>)` returns the supplied occurrences in files with an explicitly enabled `python.future-annotations` compiler feature. It reads only `compiler-feature`; it does not inspect imports, source order, enclosing statements or linter traversal state.

The Python frontend computes the module mode once from a valid absolute initial future-import prefix, after an optional initial docstring. The Python relation reader projects that existing module property. Late, nested, relative and rejected directives cannot invent a valid mode. A known Python file without the mode emits an explicit disabled row. An unsupported provider is unknown, not a disabled row.

This feature is not annotation evaluation timing. Python 3.14's native deferred annotations are independently described by `language-target`; a default expression or decorator is not deferred merely because the module has a future feature. `annotation-evaluation-context` classifies actual annotation positions. Stub typing policy and generic alias/type-parameter evaluation remain separate concerns.

The independent contract follows the [Python language reference](https://docs.python.org/3.14/reference/simple_stmts.html#future-statements). Historical evidence under `evidence/independent-read-origin` contains exact old helper snapshots; those receipts do not certify the corrected source or its new relation dependency. Frozen benchmark reports and pinned reference arrays are unchanged. Raw reference differences remain visible and require the existing evidence-backed discrepancy policy.
