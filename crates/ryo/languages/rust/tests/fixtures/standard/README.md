# rust standard

## Source

Rust has no standard that defines a syntax tree: the Rust Reference's grammar names no nodes, and rustc's AST is not a published
format. The language's kinds are those of rust-analyzer's parser, the parser of record for the shape of a Rust file's tree, and this
directory pins what that parser makes of each input.

- `syntax_kind.rs`: the generated `SyntaxKind` enum of the `ra_ap_parser` crate (github.com/rust-lang/rust-analyzer, `MIT OR
  Apache-2.0`), copied unchanged. It lists the tokens (punctuation, keywords, literals, trivia) and then the nodes of the Rust syntax
  tree in `SCREAMING_SNAKE_CASE`; the language names them in kebab case.
- `.rast` files: the tree rust-analyzer's parser prints for a file, recorded once and never edited. They are printed by the `parse`
  function of `ra_ap_parser`'s own tests (`src/tests.rs`: `LexedStr::new`, `to_input`, `TopEntryPoint::SourceFile.parse`,
  `intersperse_trivia`), run over the file with edition 2024: one line for each node (`KIND`) and each token (`KIND "text"`), indented
  by depth, white space and comments included, then one `error <offset>: <message>` line for each error. The language's tree of the same
  input, printed the same way, must equal the pinned tree line for line (`adapter_test.rs` of the adapter); the `error` lines are
  not part of the tree and are held by the `syntax_error` relation.
  - `<relation>.rast`: the tree of `relations/<relation>/input.rs`.
  - `corpus/<file>.rast`: the tree of `corpus/<file>.rs`, for every file of the corpus.
  - `suite/valid/<file>.rast` and `suite/invalid/<file>.rast`: the tree of the same path of the suite. 403 of them are byte for byte the
    `.rast` rust-analyzer publishes for that test (`test_data/parser/{ok,err,inline/ok,inline/err}`). The other 27 are printed by the
    same function from the repository's copy of the input, which differs from the published one by white space only (a final newline
    added, trailing blanks removed by the repository's hooks), so the published `.rast` does not apply to it.

## Version

`ra_ap_parser` 0.0.347 (and `ra_ap_syntax` 0.0.347 with it), the version the language's Cargo.toml pins exactly; edition 2024
(`Edition::CURRENT`), the one the language reads. No exception is stated: every pinned file is held to its tree.
