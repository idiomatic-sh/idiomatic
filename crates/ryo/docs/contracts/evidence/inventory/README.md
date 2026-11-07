# Concept inventory

Queries over the ryo source that list every concept directory, what it defines, what it imports and what imports it, and the names defined twice. They exist so a structural decision, such as moving code into a new crate, is made from what the code defines and not from line counts: a proposal to add a kernels crate would have moved functions that crates/operators already declares, and the duplicate-name tables show that overlap.

A concept directory is a directory under `crates/*/src/{domain,application,inbound,outbound}/` that holds `service.rs`, `types.rs`, `interface.rs`, `adapter.rs` or `use_case.rs`. Production lines exclude `*_test.rs`, `*_test/`, `test.rs`, `tests/`, `fixtures/` and `fixtures.rs`.

## Regenerate

Run from the repository root with a release `ryo`; each command prints one JSON answer, saved as `raw/<name>.json` without its `unread` and `shown` keys. `render.py` writes the Markdown from `raw/`.

```text
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/files.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/items.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/functions.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/dependencies.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/crate-imports.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/enum-members.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/implementations.ryoql)" --limit 1000000 --format json
ryo query run --root crates/ryo --text "$(cat crates/ryo/docs/contracts/evidence/inventory/traits.ryoql)" --limit 1000000 --format json
python3 crates/ryo/docs/contracts/evidence/inventory/render.py crates/ryo/docs/contracts/evidence/inventory
python3 crates/ryo/docs/contracts/evidence/inventory/registries.py crates/ryo/docs/contracts/evidence/inventory crates/ryo/crates
```

| Query | Answers | Failure it prevents |
| --- | --- | --- |
| `files.ryoql` | production lines per file, with crate, layer, directory | a concept sized by guess, or an empty one counted as real |
| `items.ryoql` | struct, enum, trait, function and type alias written with a visibility | a concept whose public surface nobody listed |
| `functions.ryoql` | every free function, any visibility | a private copy of another crate's function going unseen |
| `dependencies.ryoql` | file to file imports inside a crate | a concept imported by code nobody knew depended on it |
| `enum-members.ryoql` | enums and their variants | a registry's kinds listed from memory |
| `implementations.ryoql` | every `impl Trait for Type` | a port nobody can name, or a kind reached with no port |
| `traits.ryoql` | every trait | an impl counted against a trait the workspace does not declare |
| `crate-imports.ryoql` | imports of another crate's path | a crate reaching into another crate's internals |

## What RyoQL could not do

- `group-count` across files (`keys=["name"]`) joined back to its rows returns no rows, or `named producer is incomplete: frontier source proof does not match the prepared program`, on the `ryo 0.1.2` binary tried. The duplicate counts are made in `render.py` from the row queries.
- A join on a path prefix: a file belongs to its nearest ancestor concept directory, found in `render.py`.
- A `static-import` target `ryo_x::layer::a::b::Item` is mapped to a concept by `render.py` against the concept directories, and a crate library name to its directory by a table in it.
- The kinds of `OperationKind` and `DerivedKind` are declared by a macro, so `defines` has no row for their variants; `registries.py` reads them by grep, marked in the section title.
- Which function implements a kind is not a relation: `registries.py` matches the kind's snake-case name against free function names, so a count is a candidate list, and a kind implemented under another name shows zero. A port is a workspace trait with two or more impls (`inherits`) implemented in a file that holds a candidate; that is file overlap, not a call graph.
