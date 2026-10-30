# idiomatic-rust

## 0.1.0

Every finding is an error: no rule and no entry sets a `level`. `module-cycles` and `variants-benched` report as errors. `shadowed-prelude` and `test-names-nothing-beside` are placed like every other rule. `error-variant-never-constructed`, `boxed-dyn-error-annotations` and `same-name-in-sibling-crates` are removed: each reported correct code (a `#[from]` variant built by `?`, a `Box<dyn Trait>` in a `Result`'s value, the names every crate's templated `cases.rs` declares), and `one-type-per-name` covers the last.

`bounded-engine-state` is added: a use case other crates run (one with a `fixtures/contract.rs`, built once and called again and again) holds interior-mutable state (`Mutex`, `RwLock`, `RefCell`, `Cell`, `OnceCell`, `OnceLock`, `LazyCell`, `LazyLock`, atomics) only in a type of its `types.rs` named for what it keeps, bounded by the constant of the same name in its `constants.rs` (`CompiledPlans` and `COMPILED_PLANS`), which a test of the hex names. A bare locked map on the use case is the shape that let concurrent lint calls evict each other's compiled plans.

`serialize-dont-format` is 0.2.0: a raw string with real newlines is read even when it quotes code in backticks. `one-type-per-name` is 0.2.0: it spans every crate of the project (the one copy of a concept lives in the values crate) and reads types written inside a `macro_rules!` body.

`directions-edge` and `directions-composition` forbid the module above (`crate::resources`, `super::super`): what an edge needs from above is handed down, and a child never reaches up or into a sibling command. `module-cycles` is no longer a trial entry.

`module-cycles` reports every cycle: a file on a cycle of written-path dependencies is a finding, and what it needs from above is handed down by its parent.

`adapter-at-root` leaves out `**/fixtures/**`: a port's test-support files may name an adapter of their own port.

The Rust policy rules a product places once: the twenty shared rules that were global to Rust, no-env-reads and field-written-never-read (rustc does not see a public item), and the trial rules module-cycles, shadowed-prelude, test-names-nothing-beside and variants-benched. A rules-only spec: it owns nothing, so its rules reach the parts placed inside it.

Rules added: `no-string-fields`, which is the language spec's rule of the same name widened to every struct of the product and moved here; `no-sync-in-public-api`, `no-vacuous-tests`, `no-sentinel-values`, `no-hardcoded-paths` (not in tests, benches, fixtures, build scripts or a composition) and `unix-behind-cfg`. The type rules (`no-bare-primitives`, `no-bool-parameters`, `no-tuple-signatures`, `no-string-fields`) walk the generic arguments of a signature or a field at any depth, `no-string-keys` and `no-string-literals` no longer exempt the SDK resources. The trial rules (`module-cycles`, `shadowed-prelude`, `test-names-nothing-beside`) were trial entries while the engine could not decide every construct.

`no-string-fields` finds text under any pointer (`Box<str>`, `Rc<str>`, `Arc<str>`, `&str`, `Cow<str>`) as well as `String`; a tuple struct of one unnamed field that is not a `String` is the newtype that names the text and may hold it.

The type rules are one rule per bare type and cover every position: `no-bare-text` (was `no-string-fields`), `no-bare-integers` (was `no-bare-primitives`), `no-bare-floats`, `no-bare-char`, `no-bare-bool` (was `no-bool-parameters`), `no-bare-tuples` (was `no-tuple-signatures`), `no-bare-keys` (was `no-string-keys`). Fields, variant payloads, parameters, returns, generic arguments, references, slices and keys; the inner field of a one-field newtype is the one holder.

`no-string-literals` exempts the format string of the format-family macros (`format`, `format_args`, `write`, `writeln`, `print`, `println`, `eprint`, `eprintln`, `panic`, `unreachable`, `todo`, `unimplemented`, `assert*`, `debug_assert*`, `info`, `warn`, `error`, `debug`, `trace`): the first string literal directly in the macro call's token tree is output text; a second literal or a literal in a normal call stays a finding. The rule now reads the string tokens themselves, so a literal inside any macro's tokens is seen (it was not before).

`client-at-adapter` leaves out `tests/bdd.rs` (the crate spec's `bdd-runner` template renders `cargo_metadata` there) and `io-at-boundary` leaves out the command line's `output/pager.rs` (the `output-pager` template spawns the user's pager), both files a template writes.

A closure with an annotated `-> bool` is a bare bool like a function (fixture `bool-in-a-closure`). A closure whose bool is only inferred (`|x| x > 1`) has no type in the syntax and is not seen: an engine gap, as no type inference is available to a syntax rule.

`client-at-adapter` leaves out the directories named by the new `apart` param, which place their own `client-at-adapter` (a language's parser crates) in a local tastemaker.toml: two placements of one rule with different params may not reach a file.

`no-reexports` leaves out `src/lib.rs`, `src/resources.rs`, `src/resources/*.rs` and `src/resources/*/contract.rs` (the SDK root, its resource modules and their contracts), the same in the sdk spec and, prefixed `**/`, in idiomatic-rust.

`client-at-adapter` reaches a port's own `types.rs` and `errors.rs`: the exclude list keeps only adapters, factories and compositions, so an infrastructure crate (the `libraries` param) in a port's plain files is a finding. Unchanged placement; the rule gained fixtures that pin it.

`import-directions`, placed six times, once for each directory (the aliases `directions-domain`, `directions-use-case`, `directions-port`, `directions-adapter`, `directions-edge`, `directions-composition`) with the targets the directory may not name in its `forbidden` param. Domain names neither inbound nor outbound; a use case no adapter, factory or other use case; a port (`interface.rs`, `types.rs`, `errors.rs`) no adapter, use case or other port; an adapter no use case of its own crate, no other adapter and no other port (another crate's use-case interface is allowed: it is the translation adapter); an edge (`src/resources/**`) no adapter and no domain service; `use_case.rs` only in `composition.rs`. The identity of a use case, a port and an adapter is read from the file's own path, so a file's own port, use case and adapter pass whether written with `crate::` or `super::`. Tests, fixtures and benches are left out. Not seen: a `super::` or `self::` path (it cannot leave the part), a path in an attribute or a macro invocation.

`no-domain-concept-cycles`: an `error` rule with no baseline over the imports between `domain/<concept>` directories of one crate, reported at each concept's module file on a cycle. `module-cycles` stays the `info` trial rule over files.

`client-at-adapter` keeps `*_test.rs` under `domain/` in its reach on purpose: the exclude list names adapters, factories, compositions, `tests/fixtures`, `tests/allocation.rs` and `tests/bdd.rs`, never a test beside the domain, so a domain test cannot name the infrastructure library (the rule's `domain-test` fixture pins it).

`no-string-literals` also reports a byte string (`b".."`, `br".."`) and a C string (`c".."`): the rule reads the tokens of the kinds `string`, `byte-string` and `c-string`. Fixtures `byte-string-literal` (a finding) and `byte-string-in-test` (a test file may hold one).

`no-runtime-fixtures` leaves out `**/src/resources/*/contract.rs`, the SDK resource contract compiled only under the `fixtures` feature to call the contract another crate keeps in its `fixtures` (the `sdk` spec places that part and leaves it out of its own entry the same way). Fixtures `sdk-contract-names-fixtures` and `sdk-resource-names-fixtures`.

**Three rules for code that hides behaviour.** `no-panicking-closure` (a closure whose body is `panic!`, `unreachable!`, `todo!` or `unimplemented!` is an `expect` written the long way; test code is exempt), `no-thread-local-outside-adapters` (a `thread_local!` is process-global state that only an adapter holds; the entry leaves out adapters, tests, benches and fixtures) and `no-callable-domain-fields` (no `dyn Fn`, `dyn FnMut`, `dyn FnOnce` or `fn(..)` field in a type under `src/domain/`: a domain value is data). Fixtures: each rule's violation and pass cases beside its rule. Edited in place: the package is frozen.

`no-sync-in-public-api` leaves out a part's fixtures (`**/fixtures/**`, `**/fixtures.rs`): test support is no caller's API, and a fixture hands its test the `Arc` it shares with the part under test (Tastemaker's `check` fixtures share the store between two runs).

**A closure handed straight to a library takes the parameters the library fixes.** `no-bare-text`, `no-bare-integers`,
`no-bare-bool`, `no-bare-char`, `no-bare-floats` and `no-bare-tuples` take the spec's `libraries` and leave out the
parameters of a closure that is itself an argument of a call or a method call in a file that imports one of them
(minijinja's `add_filter(name, |value: &str| ..)`); `client-at-adapter` keeps such files to adapters, factories and
compositions. A closure of our own, bound to a `let`, a field or handed to a call in a file that imports no library, keeps
its parameters a position. Fixtures `closure-of-our-own` (violation) and `closure-handed-to-a-library` (pass) of each rule;
`bool-in-a-closure` stays a violation. The five placements leave out `**/tests/fixtures/**` as `no-bare-text` does: a
fixture's input is a language's test data, not code, and reading its imports would make a rule undecided on an input written
with syntax errors.

**`client-at-adapter` reads production code only.** Its placement leaves out test code and test data (`*_test.rs`,
`fixtures.rs`, `tests/`, `benches/`), which set a scene up with any library (a scratch directory from `tempfile`), an
edge's `schemas.rs`, whose wire shapes are a client library's own types (`serde_sarif`'s log), and the command line's
`src/languages.rs`, which links the languages the build carries through the SDK.

`no-string-literals` exempts a span's name: the first literal of `span!`, `info_span!`, `debug_span!`, `trace_span!`, `warn_span!` and `error_span!` names the span in the trace that leaves the engine, as a format macro's template does (fixture `span-name`).

`no-string-literals` sees a format macro's template inside another macro's tokens (`vec![format!("..")]`), where the inner macro is not parsed as a call: a token tree whose previous sibling is a `!` whose previous sibling is the macro's name holds the template; any other literal there is still a finding (fixtures `format-in-a-macro`, `literal-in-a-macro`).

`import-directions` places `directions-domain`, `directions-use-case`, `directions-port`, `directions-adapter` and `directions-application` here again, over `**/src/...`, and nu-hex no longer places them. A spec's rules never reach the files of the parts it places (a concept, a use case, a port, an adapter), so under nu-hex the five entries reached only the namespace module files and a domain file naming `crate::inbound::...` passed. Scenarios `domain-in-a-concept-names-inbound` (fails) and `domain-in-a-concept-names-domain` (passes) place nu-hex beside this spec and hold the direction inside a nested concept.

`stdio-at-presentation`, at `error`: only the command line's `output` module and its `timings` report name `stdout` or `stderr` of `std::io` or `anstream` (called, passed as a writer or imported); `main.rs`, tests, fixtures and benches are left out. The panic report every command line installs moves into `output::panicked` for it (cli 0.10.0).

`hex-cohesion`: an `error` rule over `**/src/inbound/use_cases.rs` and `**/src/inbound/use_cases/**`. The use cases of one core join when they name the same outbound port, domain concept or application service (resolved `source-dependency`, tests, fixtures, acceptance steps and benches left out); a core whose use cases fall into more than one group is reported once, at its `inbound/use_cases.rs`. Scenarios `two-groups` (fails) and `one-group` (passes). Zero findings in tastemaker, asymptote and ryo.

`fixtures-reach`: a file the `fixtures` feature builds names another crate's `fixtures` only when the crate's `[features] fixtures` enables `<dep>/fixtures`; the entry reaches `Cargo.toml` too, where the feature list is read.

Rules added: `no-inline-modules` (error) over the role files (`adapter.rs`, `method.rs`, `interface.rs`, `types.rs`, `errors.rs`, `dtos.rs`, `composition.rs`, `command.rs`, `use_case.rs`, `service.rs`, `item.rs`, `bench.rs` and their `_test.rs`), a module holding only traits (a seal) excepted; and `collection-directory` (warn, `max-items = 3`) over every Rust file.

`test-imports`, at `error`, over `**/tests/**/*.rs`, the colocated `acceptance.rs` and the manifests: a crate's integration tests name their own crate and, of another crate of the repository, only its `fixtures`, its `types` and `errors`, its `factory`, an SDK resource's `contract` and a language's `linked` (guard 2 of the test guards, owner's option C, 2026-10-27).

Rules added, at `error`, for the shape of data: `mirrored-types` (no two structs of one crate with the same three or more named fields), `one-type-per-name` (no two visible structs or enums of one crate share a name), `no-packed-strings` (no field holds values joined by a separator), `no-format-versions` (no format version constant, field, type or `-vN` name), `no-whole-state-ports` (no `Image` across a port of `outbound/`) and `composition-opens-nothing` (a `composition.rs` calls no `open` or `connect`).

Rules added, at `error`, outside tests, benches and fixtures: `serialize-dont-format` (TOML and JSON through their serializer), `ordered-output` (no hash map or set in a serialized type), `stream-discipline` (no trace writer falls back to a file in the working directory), `digest-not-stamp-alone` (no key made from a modification time) and `stable-ids` (no id made from an enumeration's position).

`no-serde-in-domain` reads every Rust source of the product but its tests: serde only in an outbound adapter's `schemas.rs` or `schemas/`, a command's `schemas.rs` under `cli/src/resources/` and the SDK's published types under `crates/sdk/src/resources/`.

`no-temp-in-use-cases` leaves out the command line's own sources (`cli/src/**`): the command line is the edge that owns the machine, as an adapter does (its `--timings=chrome` trace goes to the temporary directory).

`directions-domain` forbids the inbound and outbound of the file's own crate (`{,::}crate::inbound`, `{,::}crate::outbound`), not of every crate: a kernel crate's public types and errors live under its `inbound/` and its ports' interfaces under its `outbound/`, which a domain may name; an adapter or a factory of any crate stays forbidden. Fixture `domain-names-a-kernel-crates-public-types` (passes).

The hex directions (`directions-domain`, `-use-case`, `-port`, `-adapter`, `-application`) forbid the command line's `output::schemas`, the one module where a typed value becomes text: a product's core and SDK hand values on typed. Held here, not in a product entry, since two placements of one rule with different params may not reach a file. Fixture `domain-names-value-rendering` (fails).

`storage-shapes-in-schemas` is placed for every Rust file but the outbound adapters and `schemas.rs` files: a string table, and the integer references into it, stays in the adapter that stores it.

`bounded-engine-state` reads every use case (`use_case.rs`, `*UseCaseImpl`) and every adapter (`adapter.rs`, `*Adapter`), not only a use case with a `fixtures/contract.rs`; a test must name the cache type as well as its bound. Fixtures: `a-use-case-without-a-contract` and `a-test-names-the-bound-and-not-the-cache` (findings), `a-bare-lock-on-an-adapter` (finding), `a-bounded-cache-on-an-adapter` (passes).

`bounded-engine-state` leaves out an atomic (a counter) and a `OnceCell` or `OnceLock` (a set-once cell): nothing grows with the calls that use them, so none needs an ignore. Fixtures: `a-counter-on-the-use-case`, `a-counter-on-an-adapter` and `an-engine-built-once` (pass), `a-lock-in-a-set-once-cell` and `a-counter-beside-an-unbounded-map` (findings).

`bounded-engine-state` follows the types of the hex's `types.rs` to any depth, not one, and holds `LazyCell` and `LazyLock` to what it holds `OnceCell` and `OnceLock` to: such a cell is not state itself, but one that holds a type from a crate the package depends on (by a path from the dependency or an import of it) is reported at the cell, because that type's own state cannot be read. Fixtures: `a-set-once-cell-holding-a-foreign-engine`, `a-lazy-cell-holding-a-foreign-type` and `a-lock-two-types-down` (findings), `a-lazy-cell-of-std-values` (passes).

`bounded-engine-state`'s passing fixtures `a-bounded-named-cache` and `a-bounded-cache-on-an-adapter` hold genuinely bounded caches: the cache drops its oldest entry before it would hold more than its constant, and the test keeps more than the bound and finds the cache at it. The rule reads the names (cache, constant, a test naming both), not whether the code bounds the cache, so an unbounded map named and tested the same way is not reported; there is no failing fixture for it.
