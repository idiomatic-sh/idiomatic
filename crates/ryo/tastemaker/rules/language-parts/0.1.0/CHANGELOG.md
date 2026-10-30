# language-parts

## 0.1.0

The first version: held at the product.

The expected rows are typed Rust: the parts are `src/fixtures.rs` with `src/fixtures/kinds.rs` (how each kind is shown), `src/fixtures/relations.rs` and, for a language with manifest items, one `src/fixtures/<case>.rs`; the sources stay in `tests/fixtures/relations/<relation>/input.<ext>` and the repositories of the project cases in `tests/fixtures/manifests/<case>/`. Edited in place (the package is frozen).

The parse and project parts are the contracts `src/inbound/contracts/{parse,project}/contract.rs`, not use cases.

The suite and the corpus are not this rule's: the workspace keeps rules off `tests/fixtures/suite/` and `tests/fixtures/corpus/` (other languages' text no rule reads), which hides them from every relation a rule reads, so a rule here could never see them. The `language` spec holds them by placement instead (`suite-cases` and `corpus-files`, which see every file the workspace governs); this rule no longer checks either, and its fixtures `missing-suite`, `suite-readme-only` and `missing-corpus` moved to that spec. The product placement no longer includes the two directories.

**The template's files are required.** A language has the concepts `syntax` (service and its test), `nodes`, `tokens` (types, constants, service, test), `language` (service and test); the constant `STANDARD` in `nodes/constants.rs` and `tokens/constants.rs`; the concept `model` (types, service, test) exactly when its parse contract's `FileModel` is not `()` (a `type FileModel` whose type is a tuple type); the collection contract of the relations; the syntax port's `factory.rs` and `fixtures/library.rs`; in each syntax adapter `schemas.rs`, `schemas_test.rs`, `types.rs`, `constants.rs`, `fixtures/walk.rs` and `fixtures/oracle.rs`; `src/fixtures/contracts.rs` and `reading.rs`; and `benches/performance.rs`, the corpus being the workload. They are held here, in one table of required paths joined to the files the rule sees, because the concepts' directories are placed by `nu-hex`, whose instances a spec of the language cannot place into. Fixtures `missing-nodes`, `missing-standard`, `missing-bench`, `missing-adapter-oracle`, `model-without-files`, `model-with-files`, `unit-model`; every other fixture gained the files.

**A test beside every project or embedded adapter**. A directory `src/outbound/{project,embedded}/adapters/<adapter>/` that holds an `adapter.rs` holds its `adapter_test.rs`, which the `adapter-contract` rule then holds to the port's contract. The embedded port is a trigger by its own directory: a language without `src/outbound/embedded/` owes nothing for it. Fixtures `missing-adapter-test`, `missing-embedded-adapter-test` and `embedded-adapter`; every fixture with a project adapter gained its test.

**The links read from the parse's notes need no manifest.** `source_dependency` and `analyzed_source` items under `src/domain/manifests/collections/` (and the collection's own `manifests.rs`) do not make a language one that reads manifests: a language that links its files from the notes of its parse (JavaScript) owes no project adapter for them. The fixture `no-manifests` gained those items.

**A native manifest adapter stands for the manifest reading adapter.** A language that reads its manifests with an adapter of its own `manifest` port, `src/outbound/manifest/adapters/<tool>/adapter.rs` (ryo's Rust provider reads Cargo manifests natively with `toml`), owes no `manifest_reading` adapter; that adapter owes its `adapter_test.rs` as a manifest reading adapter does. Fixtures `native-manifest-adapter` and `native-manifest-adapter-untested`.
