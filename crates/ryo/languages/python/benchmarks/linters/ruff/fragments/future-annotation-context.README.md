# Scoped annotations future state

`future-annotation-context(expressions=<id: node, file: file>)` returns expressions visited with Ruff 0.16.5's annotations future flag enabled. It uses the exact import AST module/member identities, including aliases and relative-import behavior; lazy imports do not enable the flag.

An import sets the flag before its own statement snapshot, so it affects later visited siblings within the surrounding restoring frame. Every enclosing statement restores its entry flags after traversal; exception handlers also restore their entry flags. Elif/else clauses and match cases do not create additional restoring frames. The nearest such ancestor of the import is therefore its lifetime carrier. An expression inherits that import only when it is inside the carrier and follows the import in source order. This also preserves deferred function-definition snapshots and prevents a class/function-local future from leaking to later module definitions.

All identities and source positions are real syntax data. No synthetic event timestamp or namespace identity is introduced. The fragment models checker flag state, not diagnostic validation: stub postponement and unsupported-syntax diagnostics remain separate responsibilities.
