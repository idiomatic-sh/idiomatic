# B019 / RUF049 contextual enum ancestry

B019 passes 92 exact oracle cases and RUF049 passes 86 on Ruff 0.16.5 with target
Python 3.10 and shipped defaults. The original 116 cases and all three previously
failing expectation arrays remain unchanged. Integration owns registration.

Evidence: `contextual-closure-worker-20261101/verification.json` and its
`profiles/{B019,RUF049}/cases.jsonl` contain 356 complete reference/candidate
records with zero unknown or unread rule/file pairs. Worker identity and Cargo
bench-target metadata are in `build/identity.json`. These are fixture results,
not a certificate for the entire Ruff default-rule inventory.

## Pinned source contract

Tag `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`:

- `ruff_python_semantic/src/analyze/class.rs`: `is_enumeration`,
  `any_qualified_base_class`, and `any_base_class`.
- `ruff_python_semantic/src/model.rs`: `lookup_attribute_in_scope` and
  `lookup_symbol_in_scope`.
- `ruff_linter/src/checkers/ast/mod.rs`: lexical binding installation,
  transient TypeScopes, and statement-analysis order.
- `ruff_python_ast/src/helpers.rs`: `map_subscript` unwraps exactly one level.
- `ruff_linter/src/rules/flake8_bugbear/rules/cached_instance_method.rs` and
  `ruff_linter/src/rules/ruff/rules/dataclass_enum.rs`.
- `ruff_linter/src/rules/ruff/helpers.rs`: `dataclass_kind` chooses the first
  recognized stdlib/attrs decorator in source order.

Qualified enum imports are checked at their original base expressions. Local
ancestor names are looked up again in the current checker context. Each eligible
method or decorated class supplies its own context; transitive local ancestry is
partitioned by `[context, file]` using ordinary class node endpoints. Intermediate
classes remain in the edge graph.

The query reconstructs Ruff's lexical checker namespaces from generic AST and
binding events. It retains current binding order, global/nonlocal declarations,
completed exception-handler restoration, and active transient TypeScope barriers.
Class argument expressions and function annotations have TypeScope evaluation
roots; defaults, decorators, and nested lambda bodies keep their own evaluation
contexts. Ordinary class attributes resolve through arbitrary-length paths by
matching every name and depth against the class namespace graph.

## Reusable resolver boundary

A consumer can supply `ordered_contexts` with these columns:

| Column | Type | Meaning |
| --- | --- | --- |
| context | Node | Unique checker analysis point, usually its declaration |
| initial_scope | Node | Lexical scope active at that point |
| point | Number | Inclusive `binding-event.ordinal` at analysis |
| file | File | Source file |

`class_resolutions` returns `(context, expression, class, file)` for local Name
and class-attribute expressions. `mapped_bases` unwraps one outer subscript.
Joining these yields contextual local ancestry edges. This resolver does not
supply the ordered, tri-state callback policy used by helpers such as
`is_metaclass`; consumers must implement that policy separately.

## Context controls

The preserved original counterexamples now pass:

- `B019/local-class-member-shadow`: a class-local ordinary Base shadows the
  module enum Base while the cached method is analyzed.
- `B019/rebound-parent-name`: rebinding Base after defining Middle(Base) breaks
  the later contextual ancestry proof, so Ruff reports the cache decorator.
- `RUF049/rebound-parent-name`: the same rebinding removes the dataclass/enum
  diagnostic.

Additional controls cover methods before and after a class-member rebind,
deferred module/function lookup, current method-name binding, all intermediate
ancestor hops, globals/nonlocals, active and completed exception handlers,
annotation-only bindings, deleted members, and header/default/decorator walruses.
