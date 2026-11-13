# Modernization symbol origins

`binding-read` and `binding-use` describe static candidate associations, not guaranteed runtime loads. A read before its first function-local binding may associate an outer declaration for shadow analysis. Python nevertheless treats that name as local for the entire callable body.

This helper combines unique associations with `resolves`/`defines` lexical ownership. It rejects an imported association whose namespace contradicts a callable-local declaration and rejects builtin inference when a callable-local declaration owns the name. It does not require identical declaration IDs: later assignments/imports in the same namespace remain distinct valid reaching definitions. Module/class execution order and global/nonlocal redirects retain their language facts.

Regression cases live beside UP007, UP045, UP050, and TC005. The exact local-alias CPython controls show UnboundLocalError, independently of the pinned linter's output.
