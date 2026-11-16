# F401 deferred-scope caller boundary

Ruff 0.16.5 invokes `unused_import` only for module and function scopes in `analyze/deferred_scopes.rs`. Class imports define attributes and do not reach that rule caller. The query now applies that boundary before calculating import use and export policy, including ordinary and asynchronous methods, nested classes, and stub classes.

All 22 original live-integration controls are preserved. The scoped actual-worker profile passes 29/29 cases with 58 complete contestant records and zero unknown/unread outcomes on worker SHA `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. The final frozen reproduction is preserved with its full report arrays in `cross-profile-cases.toml`.

Candidate-only replay of that exact source under the complete frozen 310-query inventory plus queued fixes matches the sealed reference report exactly: two complete records, zero unknown/unread or skipped pairs. Evidence: `f401-final-full310-sol/{cases.jsonl,result.json}`; scoped evidence: `f401-sol-seven/cases.jsonl`. This does not assert complete 413-rule parity.

## Ruff semantic-model policy in the rule

The Python provider reports lexical facts; three Ruff 0.16.5 policies that change which imports count as used live in the rule query, because a provider carries no linter policy.

- Alias reads mark submodule imports. `ruff_python_semantic/src/model.rs` `resolve_submodule`: a load of `import M as N` also references a `SubmoduleImport` bound in the same scope before the load, when the last segment of M names it and both share M's first segment (`import pandas as pd` beside `import pandas.tseries.holiday`). The query's `submodule_uses` implements it for plain `import` statements only.
- Forward references resolve in the module scope first. `model.rs` `resolve_load` in a forward reference (a string annotation or a future-annotations annotation) prefers a live module binding over the lexical one. The query routes reads through the shared `python/ruff/deferred-read-events` fragment, as F811 and F841 do, so a local import read only by a string annotation stays unused when a module binding wins.
- `global` and `nonlocal` imports are never reported. `checkers/ast/mod.rs` flags an import of a `global` name in a function scope, and of a `nonlocal` name when an enclosing function binds it; `unused_import.rs` skips flagged bindings. The query matches the declaration against the import's own enclosing scope, not the provider's owner scope. A module-level `global` never exempts. An import in a class body is never checked at all, because Ruff runs the rule on module and function scopes only (see the deferred-scope section above), `global` or not.

An alias read also counts as a use at its position, so the submodule binding it marks passes that use to a later rebinding of the same name through the same replacement and inheritance step as any other use (`checker/mod.rs`, copy of references on rebinding).

The cases `submodule-*`, `forward-reference-*`, `future-annotation-*`, `global-*`, `module-level-global-import` and `nonlocal-*` pin each clause against Ruff.

A read resolves to the latest binding of its name in the nearest scope that has one, in any branch (`model.rs` `resolve_load` finds the scope's binding before looking outward), so `import os` / `def f(): if c: import os / else: print(os)` leaves the module import unused. The query takes this from the shared `python/ruff/ordinary-read-uses` fragment, which calls `python/ruff/lookup-read-events` (the one definition of Ruff's name lookup, also used for deferred reads by F811, F841 and F821): reads from comprehension, lambda and nested-function scopes look through their enclosing scopes.
