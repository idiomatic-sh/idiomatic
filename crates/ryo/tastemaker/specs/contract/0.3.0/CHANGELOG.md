# contract

## 0.3.0

A scaffolded contract compiles against the engine's traits as they are. `Parse` renders the explicit `ParseUseCase` methods `begin_semantics` (the buffer-filling `SemanticSlot` the language leaves empty), `completion_requirement` (`Independent`), `extend_file_model` and `recognizes_explicit` (`Declines`), and answers the relations `standard_library_import` and `token_link` as `NotYetRead`. `Project` renders `project_entries` (which fills the engine's buffer), takes the `observed_entries` of its request into `Resolved`, iterates `manifests()` by copy and answers `project_namespace_membership` as `NotYetRead`. The rendered fixtures carry them, and the scenarios that mutate a rendering expect one more finding for each member it lacks. A throwaway language scaffolded from `language` 0.4.0 compiles with `cargo check --all-targets`.

Edited in place: places `no-runtime-fixtures@0.2.0`, where it named its earlier version, so a product that places it locks one version of the rule: two versions of one rule share its id, and the earlier entries ran the later query.

Edited in place: the module file of a directory is `<dir>.rs` beside it, never `<dir>/mod.rs` (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0): a group of the part's own cases is `fixtures/<group>.rs`.

Edited in place: the `contract-fixture` template's scenario places `fixtures/links.rs`, the form the spec places.

## 0.2.0

A contract part may hold readers: `readers.rs` (the list of concepts), `readers/<concept>.rs`, and for each concept `composition.rs` (the readers bound to the grammar adapters that serve them, the one place that may name an adapter), `service.rs`, `types.rs`, `constants.rs`, the proof `composition_test.rs` (and its groups) or `service_test.rs`, and `fixtures.rs` with `fixtures/<file>.rs`. The part may also hold `constants.rs` and `service.rs` of its own. All are optional (`count = { min = 0 }`), so a contract without readers is unchanged. New templates `reader-list`, `reader-module`, `reader-composition`, `reader-fixtures`, `reader-fixture`. Fixtures: `contract-with-readers`, `reader-composition-without-its-role`.

Also `fixtures.rs` and `fixtures/<group>/mod.rs` of the part itself, optional, for a proof grouped by concern: templates `contract-fixtures`, `contract-fixture`.

## 0.1.0

The first version: a language's `parse` and `project` are contract parts, not use cases. A contract part is `contract.rs`, which imports the engine's trait (`ryo::languages::Parse`) and implements it for the struct `<Part>Contract`, and `contract_test.rs`, the language's own cases in the one test shape. The `trait` param names the trait, and the rule `contract-implements` holds that the file imports that exact path and implements it. The language spec places the two contracts and their module files; the part has no interface, types, errors or fakes of its own. Fixtures: the complete contract, no test, no contract file and a contract that does not implement its trait, the four of the rule and the templates' own.

The `contract` template's struct names no visibility: a contract may be `pub` or `pub(crate)` (a template that names a visibility is compared exactly). Fixture `crate-visible`.

The `contract-test` template renders no runner: like every test template, it holds the `CASES` table and the subject, and the author writes a `run(CASES, Word::<word>, subject)` runner exactly for each case word the table has rows of. `test-runners-match-rows` is placed beside `test-shape` and `case-expectation`. The fixtures' tests keep only the runners of the words they have rows of; new fixture `test-with-rejects-row-without-runner`.

**Per-file methods fill the engine's buffers.** New rule `contract-fills-buffers` (contract.rs): a method of the trait impl that takes a parameter beside `self` returns no `Vec`, `String`, `Box`, map or `Self::<Assoc>` by value; the engine owns the per-file buffers and the language fills them. Fixtures: `fills-a-buffer`, `methods-without-a-file`, `derived-state-in-a-result` (pass), `returns-a-vec`, `returns-a-string`, `returns-a-result-of-a-box`, `returns-an-associated-type` (violation). Edited in place: the package is frozen.

**A scaffolded contract compiles and answers honestly.** The `contract` template renders the whole implementation for each engine trait: `Parse` holds the syntax port, recognizes no file, builds no tree and has no kind, reads `syntax_tree` through its item and answers every other relation `NotInThisLanguage` (`NoSuchConstruct`); `Project` names no manifest and answers every manifest relation `NotInThisLanguage` (`NoManifests`). The struct is `pub(crate)`, and `ParseContract::new` takes the port. The `contract-test` template renders cases that pass: the engine's refusals (and, for `Parse`, a file the language does not recognize, refused as unsupported; for `Project`, a repository with no manifest) with their runners, with no `todo!`. Fixtures: `example-project` of both templates, `public-struct`, `without-subject`; the others regenerated from the rendering. `crate-visible` is gone: the template names `pub(crate)`.

**The scaffolded contract answers `call_argument` and `resolves`**: the `contract` template renders the two `Parse` methods the engine added, each absent with `NotYetRead` like every other relation the scaffold does not read; the rendered fixtures carry them. Edited in place.
