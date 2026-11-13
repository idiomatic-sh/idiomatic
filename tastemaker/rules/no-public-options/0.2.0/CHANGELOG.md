# no-public-options

## 0.2.0

The `internal-crates` param names the library names of the crates a product splits its own engine into, which only its engine crates depend on (ryo's engine domain and store crates): a crate a workspace package binds under one of those names has no public API, so its files are not read; its `pub` items are the engine's internal surface. Fixture: `internal-crate-option` (the internal crate's field is not read; the same field in the depending crate is a finding). Everything else is as in 0.1.0.

## 0.1.0

The first release. It replaces the `no_public_options` dylint lint of the
deleted `crates/tastemaker/lints/architecture` library ( the custom lints
become Tastemaker rules defined in TOML): a named or positional field written plain `pub`, in
a struct written plain `pub`, whose type is `Option` (local policy). The
devtool starter's shared rule block places it with `include = ["**"]` and
`exclude = ["**/dtos.rs"]`: a wire representation is kept inside its adapter by
`dto-visibility` instead.

## Differences from the lint

- The type is read as written: a path type whose last segment is `Option`, so `Option<T>`,
 `std::option::Option<T>` and `core::option::Option<T>`. The lint resolved the type, so it
 also saw an `Option` behind a type alias. That is the gap: a private alias in the same
 module (a public alias is a `no-reexports` finding, and a domain's `types.rs` and
 `errors.rs` hold none, `no-domain-aliases`).
- A type of another crate named `Option` would be a finding; none is in use.

## Moving up from the no_public_options lint

- Add the shared rule block's entry to the product's `tastemaker.toml`:
 `{ package = "idiomatic.arch", version = "0.1.0", rule = "no-public-options", include = ["**"], exclude = ["**/dtos.rs"] }`.

## Fixtures

Each `//~ no_public_options` marker of the lint's UI fixtures is one failing fixture:
`public-option` (a named field), `qualified-positional` (`pub std::option::Option<u32>`) and
`beyond-types` (a public struct outside `types.rs`). Its passing cases pass:
`private-option` (private, `pub(crate)` and `pub(super)` fields, `Vec<Option<u32>>`, a
local), `semantic-choice` (an enum), `private-struct` (a public field of a private struct)
and `dtos-file` (a `dtos.rs`, outside the entry).

The `timings-flag` fixture holds the `cli-timings` template's rendered `src/timings.rs`, so
the flag every CLI will carry is proven to pass this rule.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
