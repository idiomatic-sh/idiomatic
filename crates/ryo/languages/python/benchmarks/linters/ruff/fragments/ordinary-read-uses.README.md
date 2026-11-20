# Uses of ordinary reads

`ordinary-read-uses(reads=<reference, file>)` gives the binding each given read or deletion uses. It is a projection of the Python provider's `binding-resolution` relation (the event in effect in each namespace on the reference's lookup path, in schedule order) plus Ruff's adjustments, which are linter policy and so stay here. A read that resolves to a deletion, a cleanup, an annotation or nothing uses no binding.

The provider owns Python's resolution: the lookup path (own namespace, enclosing functions skipping classes, type-parameter scopes, the module after `global`), the schedule order (a function or lambda body runs after its enclosing scope, so its reads see that scope's final bindings), lazy type-parameter bounds and defaults, and global/nonlocal routing (a routed store is an event of its home namespace, with `lexical` naming the declaring one). The fragment applies, in order:

- R0: a deletion resolves in its own lexical namespace only (depth 0).
- R1: an event whose `lexical` differs from its row's namespace is ignored there (the declaring scope holds it, Ruff's model); a declaration row (`global`/`nonlocal` outside the module) takes as its namespace's answer the latest valid event of the name whose code sits in the declaring namespace before the reference's point (a lazy read's point is the end of the file).
- R2: events Ruff does not count are stepped over: conditional and handler-owned deletions, events in type roots, `.py` module and class annotations, delayed function and stub annotations, stub module annotations of builtins, and module-level `global` statements. The predicates are the ones `lookup-read-events` uses (`conditional-deletions`, `annotation-evaluation-context`).
- R3: a handler's cleanup restores the event before the handler's bind.
- R4: a declaration answer not reached through R3 is dropped.
- The nearest surviving namespace (smallest depth) answers; bindings, `global` and `nonlocal` are returned.

## Cost

Rows that need no step (a valid bind, annotation or deletion whose code sits in its namespace, or a type parameter) answer directly: one join with the relation and two antijoins. Only the (namespace, name) groups of rows that step are read from `binding-event`: their events are paired by `ordered-match`, each ignored event points at its predecessor, each cleanup at the event before its handler's bind, and one `follow` takes each row to the event its pointers end at. Declaring namespaces (rows of `global`/`nonlocal` statements) have their own stream keyed by `lexical`; they are rare, and empty streams cost nothing under demand-driven evaluation.

There is no fallback join. A chain of events (reads included) of one name longer than 2^20 (1,048,576 in one file) would not reach its event in 20 rounds, so the Python provider declares such a file incomplete (`NAME_CHAIN_LIMIT` in `binding_analysis`): every rule that reads the binding facts answers Unknown for that file and never reports no finding. The bench's growth plan (`composition/growth.rs`) times F401, F841, F811 and F821 over every operation at 500 and 1,000 repetitions and fails on a doubling bound on time and on memory.
