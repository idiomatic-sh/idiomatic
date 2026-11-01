# RUF051 — if-key-in-dict-del

Pinned policy: Ruff 0.16.5 `crates/ruff_linter/src/rules/ruff/rules/if_key_in_dict_del.rs`, `typing.rs::is_dict`, and semantic `only_binding` / `match_builtin_expr`.

The query requires one `if` membership comparison, one subscript deletion, matching dictionary names, and matching name or constant keys. Constant matching also preserves the AST literal kind (boolean/integer/float are distinct). Any `else`/`elif`, extra statement or deletion target is excluded. The shared type helper follows Ruff’s single-binding requirement, initializer associations, annotations, and checker-time builtin/typing lookup rather than name spelling.

69 retained differential cases include the tagged upstream fixture, typed and nested assignments, prior deletion/cleanup/global/nonlocal events, annotation versus initializer precedence, builtin-import phase, deferred stub/future annotations, generic scopes, and class defaults inside active functions. Full 413-rule reference arrays remain in `full-cases.toml`; the standalone profile projects RUF051 plus invalid-syntax.

Actual worker2ea evidence and exact executed transitive module registries are recorded in `results/ruf051-2026-11-01.json`. Existing consumers FURB132101, PERF40265, RUF008100, FURB129106 and lookup consumers B008129, PYI019139, PYI03472 pass retained references. This is bounded differential evidence.
