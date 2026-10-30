# lint-call-count-test

## 0.1.0

The call-count row of the whole per-file path: one test file, `tests/call_count.rs` of the core, that lints the fixtures' corpus as it is and with ten times the rows, and checks the facts port is called as often for both.

The content follows the language API the command line links (`ryo_rust::linked()` and
`LanguageHandles`), as `cli/tests/call_count.rs` does.

The Rust language is handed to the language factory as `LanguageHandles::from(rust)`: the SDK's `Language` has no method that gives its engine outside its `fixtures` feature.
