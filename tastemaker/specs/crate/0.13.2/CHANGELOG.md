# crate 0.13.2

Places by role what a product listed file by file: the crate's grouped integration tests, `tests/test.rs` and `tests/{*group}/` at any depth (`mod.rs`, `test.rs`, `fixtures.rs` and its `fixtures/` tree), the proof of its case runner `src/cases_test.rs`, and its license `LICENSE*`, each with `count = { min = 0 }`. The license pattern writes out no file name, so a spec that places the license with a template of its own (the command line's MIT text) is the more specific entry and places it. A test group is never beneath `tests/fixtures/`, which holds the inputs `fixture-tree` places. A new crate-local template, `integration-test`, places a group's test and test support. Scenarios `grouped-tests-license-and-cases-test` (nothing closed) and `file-of-no-role-in-a-test-group` (closed). Everything else is as in 0.13.1. 0.13.1 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility. A test group's module is `test-group-module@0.2.0`, held by `test-group-namespace@0.1.0`: it declares modules only, and a group that only gathers nested groups is a namespace.

Edited in place: places `collection-members@0.2.0` with `item-trait = []`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing).

Edited in place: the module file of a directory is `<dir>.rs` beside it, never `<dir>/mod.rs` (owner, 2026-11-11; `mod_module_files` denied by cargo-workspace 0.13.0). A group of the integration tests has its module file at `tests/<group>.rs`, at any depth, never `mod.rs` (`test-group-mod-rs-is-closed`); a file of no role in a group is refused by `test-group-namespace` where it was closed.

# crate

## 0.13.1

An engine library crate, an application layer with no use case of its own (ryo's execution store, which the engine above it calls), keeps the test support `src/cases.rs` as a core does: its `src/application.rs` answers the `cases` condition beside `src/inbound.rs`, `src/domain.rs`, `src/resources.rs` and `src/main.rs`. Fixture `engine-library-application-no-use-case-answers`. Everything else is as in 0.13.0, which stays because the asymptote and tastemaker locks still pin it.

## 0.13.0

Edited in place: the `cases` template's `run` runs every row of a word, as the crates' runners do: a red row, or a subject that panics, is collected with its row's name, and one panic after the loop names every red row, so a red row no longer hides the rows after it. The rendered scenarios carry the new body.

Edited in place: `crate-readme` shows Usage, with its one line and runnable example, only for an SDK crate (the crate that places `entries`), the only crate a program calls; every other crate is internal and its README has no Usage section, in place of a hollow `use <crate> as _;`.

Edited in place: `crate-root` declares a command line's `pub mod edge;` beside `pub mod resources;`, the module of the `cli` spec's `edge` template (`src/edge.rs`).

The crate places `orphan-rust-files` (now at error): a `.rs` file no `mod` line reaches is a finding in every Rust
product. Each scenario gives the workspace root its crate is a member of, as Cargo needs to read the crate.

`cargo-package` writes `repository.workspace = true`: every crate takes the repository from its workspace.

For a shipped command line crate (`distribution = "shipped"`), `cargo-package` writes `[package.metadata.binstall]`:
`cargo binstall <package>` fetches the archive of the GitHub Release tagged `<package>-v<version>`
(`<package>-<target>.tar.gz`, `.zip` on the two Windows targets), which holds the bare command.

`crate-root` includes the README as the crate's documentation only in a documentation build:
`#![cfg_attr(doc, doc = include_str!("../README.md"))]`, which clippy's `doc_include_without_cfg` (on in
`cargo-workspace` 0.11.0) asks for, so no other build reads the file. Outside a documentation build the crate is
documented by its Cargo description, `#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]`, which keeps rustc's
`missing_docs` satisfied; every crate's manifest carries a `description` (clippy's `cargo_common_metadata`). Nothing else
changes. A product moves to this version by changing the `version` of each `crate` placement and replacing the first
line of each crate's `src/lib.rs` with the two lines.

Scenarios named `*-reaches-a-*-part` prove the crate's rules read the files of the parts placed beside it (nu-hex parts and a cli command part).

## 0.12.0

`crate-bench` renders a language's growth check guarded: `complexity()` is called only when the bench is run with the `--bench` argument (`BENCH_FLAG`), so `cargo test` builds the bench without timing it and only `cargo bench` makes the check.

`fixture-tree` also leaves out `tests/fixtures/suite/**` and `tests/fixtures/corpus/**`: the language spec places the suite cases and the corpus files, so no file is placed twice.

`fixture-tree` leaves out `tests/fixtures/suite/README.md`: a language's pinned-suite README is placed by the language spec's `language-suite`, so the crate and the language no longer both place it (fixtures `suite-readme-unplaced`, `fixture-data-placed`).

**The crate's tests, rebuilt**, edited in place:

- The `cases` template compiles: a refusal selector answers a `Selection` (`Matches` or `Differs`), never a bare `bool`, and `run` reports a row by `case.name.0`. Every `cases.rs` of the products is the rendered template; the fixture `bare-bool-refusal` holds the old selector as a finding.
- `crate-readme` lists each `concept` instance under `## Domains` beside each `hex` (`instances.concept`): a nu-hex crate's domains are its concepts, so the README the template renders agrees with the spec's own line ("a line for each of its domains") and with `tm workspace explain`. A crate with no domain has no section.
- `cargo-package` no longer requires `thiserror`: the compiler already refuses an `errors.rs` that derives it without the dependency, and a dependency nothing uses is the unused-dependency tool's finding, so the manifest names it for no crate. The fixture `manifest-without-thiserror` of `nu-hex` is now `manifest-without-errors` and passes.
- A crate's tests are of three kinds: the unit test beside its primitive, the contract a port
 exports, and the crate's behaviour through its surface with only the stores' connections
 faked. The spec's description says so, and the principle once (one small alphabet of
 primitives, the same five at every level, a file that is none a finding).
- `end-to-end` and `end-to-end-feature` are gone, with their fixtures. A `tests/end_to_end.rs`
 or `tests/<feature>/end_to_end.rs` is now a `crate#closed` finding; what it held is a feature
 scenario. The `readme()` test `end-to-end` wrote for a command line crate (its product
 README's Usage, run by `trycmd`) is the `cli` spec's to place.
- `bdd-feature` places `tests/<feature>.feature`, one per use case, and writes no stub scenario:
 a `Rule:` and a `Scenario Outline` tagged with one case word whose empty `Examples` table has
 the unit rows' columns (name, input, expected), so a scaffold fails the runner until it says
 something. The template is compared by the file's existence: Tastemaker has no Gherkin parser,
 so the shape is the runner's to hold. `bdd-steps` is new: `tests/bdd/<feature>.rs`, one steps
 file per feature. `bdd-runner` (`tests/bdd.rs`) is now four functions, `features` (the feature
 files next to it), `check` (every scenario and outline carries exactly one of `@succeeds`,
 `@refuses`, `@fails`, `@rejects`; the features state a success and a refusal), `run` (one
 feature, failing on a skipped step, a failed step and no passed step) and `main`, and
 declares each steps module with `#[path]`. It is placed exactly when the crate holds a feature
 (the `features` condition, answered by `present`); a crate with no use case has no feature
 and no runner. The names are `tests/<feature>.feature` and `tests/bdd/<feature>.rs` because
 the capture is the spec's own, so the runner's `captures.feature` lists the features only.
- `cases` is new: `src/cases.rs`, the test support (`Case`, `Word`, `Expect`, `run`) every unit
 test of the one shape calls, with no dependency, declared by the root as
 `#[cfg(any(test, feature = "fixtures"))] pub mod cases;` (a core that exports contracts, with
 `present.outbound`, adds `feature = "conformance"`: a contract names its rows' types, and no feature may
 enable `fixtures`, `fixtures-feature`, so `--features conformance` alone compiles). It is placed in a core, an SDK, a
 command line and an adapter-crate (the `cases` condition, answered by the new owned rule
 `tested-by-cases`: `src/inbound.rs`, `src/entries.rs`, `src/main.rs`, or a hex's
 `adapters.rs` with no `interface.rs` beside it).
- `crate-root` declares no `types` and no `errors` for a core
 and declares `cases`. `cargo-package` writes the `bdd` test target and `cargo_metadata`,
 `cucumber` and `tokio` only for a crate with a runner.
- Rules: `bdd-runner` is rewritten function by function (the earlier rule matched the whole
 runner as one tree, which a runner of four functions and per-feature modules cannot be);
 `bdd-members` is new (a feature and its steps file are paired, and a feature names a use
 case, `src/inbound/use_cases/<feature>.rs`; the reverse, a use case with no feature, is the
 nu-hex's); `tested-by-cases` is new; the `present` rule answers `features` now.
- Fixtures: `complete` has no tests; new passing `with-features`, `runner-file-expect`,
 `adapter-crate-with-cases`, `cli-without-bdd`; new failing `feature-without-steps`,
 `steps-without-feature`, `feature-without-use-case`, `runner-missing`,
 `runner-without-features`, `runner-undeclared-steps`, `runner-skips`, `runner-untagged`,
 `runner-without-target`, `cases-missing`, `cases-in-plain-crate`, `cases-undeclared`,
 `cli-with-bdd`; gone: `undeclared-feature`, `stray-feature-file`. `split` splits the bench
 only.

`cargo-package`: a command line crate depends on `indicatif`, which the `cli` spec's
`output` draws its progress line with, and dev-depends on `snapbox`, which its
`command-tree-contract` compares the command surface with.

`cargo-package`: with `distribution = "shipped"` the command line crate also declares
the `distribution` bench, `../distribution/benches/bench.rs`, `harness = false`, behind the
`distribution` feature and with `test = false`: the targets' budgets, which the
performance job runs and no test does.

The owned `captured-workload` rule requires a recorded port's replay workload; its fixtures include nested crate ownership and missing replay.

A crate's manifest and library root declare what its placed files need, so a product
generated from the `devtool` starter compiles without hand edits:

- `cargo-package`: a command line crate (one the `cli` spec places `src/main.rs` in)
 depends on `anstream`, `clap`, `mimalloc`, `serde` and `serde_json` and, for its README
 test, `trycmd` in `[dev-dependencies]`; a crate holding a hex, a macro crate too, depends
 on `thiserror`,
 which every hex's `errors.rs` derives with. 0.11.0 wrote no `[dependencies]` at all.
- `crate-root`: a command line crate declares `pub mod commands;`, `pub mod outcome;` and
 `pub mod output;`, and every crate declares each of its hexes, `pub mod <hex>;` (a macro
 crate `mod <hex>;`, since a proc-macro crate exports only its macros). 0.11.0 declared
 no module, so no scaffolded file compiled.
- `crate-bench`: the budget helpers are written only beside a bench that uses them (a
 composition's or a workload's); a crate with neither, a command line crate whose
 commands are benched by the `cli` spec, benches only its `main`, which `dead_code =
 "deny"` otherwise refuses.

Fixtures: crate-bench's `no-budget` is now `main-only`, which passes; new failing
`bench-without-budget`. The `cli` spec's `cli-without-dependencies` and
`output-undeclared`, and the `devtool` starter's `hex-undeclared` and
`hex-without-thiserror`, are the failing cases of the manifest and the root.

**A crate places no composition root** (ordinary library crates have no
composition module and no `Services`; `composition` is placed by `cli` and macro crates
only). The `composition` entry and its template are gone, so a
`src/composition.rs` in a library crate is a `crate#closed` finding. Composition is the
edge's, and each edge's spec places its own: `cli` one per command, `sdk` one per function,
`macro` 0.4.0 the macro crate's `src/composition.rs`, and `idiomatic.ryo`'s `language` a
provider's. None declares `Services`: the shared `hollow-services` 0.1.0, which those four
place, refuses an empty one. `crate-bench` and `end-to-end` no longer
ask a crate with a composition for a `compose` bench and a `composes_its_services` test,
which named `composition::Services::default()`; the budget helpers are written beside a
workload.

**`crate-root` declares what the specs beside it place**:

- each hex's first directory under `src/`, once, so hexes in a collection
 (`src/checks/<check>/`) are declared by their collection's module, `pub mod checks;`,
 where the template used to ask the root for `pub mod <check>;`;
- a crate that composes for itself (a `composition` entry is present: a macro crate, a
 language provider) keeps its hexes behind its composition, so its root is asked for
 `mod <hex>;` and may add `pub` or attributes;
- an SDK crate's `pub mod entries;` and `pub mod vocabulary;` (`sdk` 0.1.0), and a command
 line's `mod languages;` when it links language providers (`cli` 0.10.0).

Fixtures: `complete` and `captured-folder` hold no composition; `no-composition-root`,
`composition-undeclared` and `private-lib-composition` are gone with the entry; new failing
`library-composition`; `bench-without-budget` is a workload without its helpers; new
passing `nested-hexes` and failing `nested-hexes-undeclared`.

**No catch-all directories**. The
`end-to-end-common` (`tests/common/{fixture}.rs`) and `bench-common`
(`benches/performance/common/{harness}.rs`) entries and their templates are gone, with the
`exclude`s of `end-to-end-feature` and `bench-workload` that kept their directories apart;
`end-to-end` and `crate-bench` no longer declare a `common/` module. A file in
`tests/common/` or `benches/performance/common/` is now a `crate#closed` finding: what it
held is filed into the primitive it is (a port's fake or a case builder in the owning
hex's `fixtures.rs` or split fixtures, a workload's own code in its `bench.rs`, the bench's
harness in `benches/performance.rs`). Fixtures: `split` declares no shared module; new
failing `tests-common` and `benches-common`.

**A fixture tree is placed**. The
`fixture-tree` template, at `tests/fixtures/{*path}` with `count = { min = 0 }`, places
every file of another workspace that the crate's tests read as input, of any kind and in
any language. Its content is empty, so it asks nothing of a file and no file is parsed to
be compared with it. Its fixtures (passing `example` and `other-languages`, failing
`missing`) are the template's own.

**A command line crate has no BDD files**. The spec's own `present` rule (a copy of the hex spec's: an owned rule is
private) answers the new `binary` condition, which holds when the crate holds
`src/main.rs`; `bdd-feature` and `bdd-runner` then count 0, so a `tests/bdd.feature` or
`tests/bdd.rs` in a command line crate is a `#count` finding whose entry says what to do:
put the scenario in the crate that owns the behaviour (its `tests/bdd.feature`); a
command's own behaviour goes in its `command_test.rs`, and what needs the built binary in
`tests/end_to_end.rs`. `cargo-package` writes neither `[[test]] name = "bdd"` nor
the BDD runner's `cucumber`, `tokio` and `cargo_metadata` for a command line crate. Fixtures: new passing `cli-without-bdd` and failing
`cli-with-bdd`; the `cli` spec's `complete` and the `devtool` starter's `complete` and
`domain-imports-cli` command lines lost their BDD files.

### Moving up from 0.11.0

A command line crate moves each BDD scenario to the crate that owns its behaviour or to a
command's `command_test.rs`, then deletes `tests/bdd.feature`, `tests/bdd.rs`, the
`[[test]] name = "bdd"` target and the runner's `cucumber`, `tokio` and `cargo_metadata` dev-dependencies. A crate's
`tests/common/` and `benches/performance/common/` files move into the primitives they are
(above).

A library crate deletes its `src/composition.rs` and `mod composition;`: what it built
moves to the edges that call it (a command's or an SDK function's `composition.rs`, a test,
a bench), which call the crate's factories. A macro crate and a language provider keep
theirs, placed by `macro` 0.4.0 and `language`, and delete an empty `Services`.

`tm workspace update` takes `crate` 0.12.0. A command line crate's `Cargo.toml`
depends on `anstream`, `clap`, `mimalloc`, `serde` and `serde_json` (`.workspace =
true`) and dev-depends on `trycmd`; a crate holding a hex depends on `thiserror`. Its
`src/lib.rs` declares `pub mod commands;`, `pub mod outcome;`, `pub mod output;` (a
command line crate) and `pub mod <hex>;` for each hex (`mod <hex>;` in a macro crate). A bench with no composition and no
workload may drop its budget helpers.

**Composes with the two crate kinds**, edited in
place while releases are frozen. A nu-hex or an adapter-crate is placed beside this spec at
the crate's directory, so a scaffolded crate must compile:

- `crate-root` declares a core's `errors`, `inbound`, `outbound` (when it has ports), `types`
 and, when present, `domain` and `collections`, with `fixtures` behind the `fixtures` feature
 (it reads `present.inbound`, `present.outbound`, `present["domain-concepts"]` and
 `present.collections`), and each hex of an adapter-crate, `pub mod <hex>;` (it reads
 `captures.domain`).
- `cargo-package` depends on `thiserror` for a core or an adapter-crate as it does for a hex,
 and declares the `conformance` feature when the core has ports, whose contracts the adapter-crates
 run (`present.outbound`).
- `crate-readme` lists each hex of an adapter-crate under `## Domains` (`captures.domain`), as it
 lists each `hex` instance: an adapter-crate's hexes are its domains, a core is itself the one
 domain of its crate and lists none. The adapter-crate branches of `crate-root`, `crate-readme` and
 `cargo-package` are told by the adapters the spec places (`captures.implementation`), not by
 `captures.domain`, which a local `tastemaker.toml`'s `domain-module` entries also fill: fixture
 `local-file-hexes`.

The fixtures that prove the new branches are the two specs' own, which apply this spec beside
theirs: `nu-hex`'s and `adapter-crate`'s `complete`, `lib-undeclared`, `manifest-without-thiserror`,
(core) `manifest-without-conformance` and (adapter-crate) `readme-without-hex`. A template fixture places a template alone, so it
cannot set `present` or `captures`.

## 0.11.0

A domain's fakes reach other crates' tests the standard Cargo way: each hex's `fixtures.rs` gates itself on `test` or
the crate's `fixtures` feature (hex 0.8.0), and this spec gives every crate that feature.

- **`cargo-package`** declares `[features] fixtures = []`. New failing fixture:
 `no-fixtures-feature`.
- **`fixtures-feature`** (new owned rule, on `Cargo.toml`): `fixtures` is enabled only from
 `[dev-dependencies]`. A finding is `fixtures` in the `features` of a `[dependencies]` or
 `[build-dependencies]` entry (target-specific tables included, inline or as a table), or
 `fixtures` or `<dep>/fixtures` in any feature of the crate's own `[features]`, so no feature
 (not even an exported contract's `conformance`) carries fixtures into production (test-only, no exception); a test running another crate's exported contract enables
 both `conformance` and `fixtures` from `[dev-dependencies]`. The `fixtures` feature itself,
 and only it, may enable `<dep>/fixtures`, so that it compiles alone when its fixtures
 stand on another crate's. Fixtures: passing
 `dev-dependency`, `fixtures-enables-dependency`; failing `production-dependency`, `build-dependency`, `target-dependency`,
 `default-feature`, `feature-forwards`, `feature-enables-dependency`.
- **`end-to-end`** and **`crate-bench`** run `composition::Services` only when the crate has a composition root.

**`composition`** is optional (`count = { min = 0 }`) and `crate-root` declares `mod composition;` only when the crate has one: composition is the edge's, one per CLI command, so the CLI crate drops `src/composition.rs`; U6 removes the library crates' own. New fixtures: passing `no-composition-root`; failing `composition-undeclared`; `crate-root`'s `no-composition` now passes.

What the rule cannot see: a dependency renamed with `package = ".."` is read by its key;
`[workspace.dependencies]` in the product's root manifest is not a crate manifest and is
not placed by this spec.

### Moving up from 0.10.0

`tm workspace update` takes `crate` 0.11.0 (with `hex` 0.8.0 and `domain-module`
0.3.0, which the devtool starter 0.23.0 names together); then add `[features] fixtures =
[]` to each crate's `Cargo.toml`, remove `fixtures` from any feature that enables it (a `conformance`
feature included), and move each `fixtures-feature` finding's dependency, or its feature, to
`[dev-dependencies]`. The CLI crate deletes `src/composition.rs` and its `mod composition;` as cli 0.8.0 says.

## 0.10.0

A crate's error types are `thiserror`
derives in its domains' errors.rs. The spec owns and applies two rules: `thiserror-in-errors`
over the whole crate except `**/errors.rs` (`thiserror` is named nowhere else), and
`error-shape` over the whole crate (no hand-written `impl Error`, and no hand-written
`impl Display` for an error type, however it is derived).

Moving up from 0.9.0: move each `#[derive(thiserror::Error)]` type, and each `thiserror`
import, into the domain's errors.rs (a test's own failure can be a
`Box<dyn std::error::Error>`); replace each hand-written `impl Error` and the `impl Display`
of an error type with the derive and an `#[error("..")]` message per variant. Keep each
cause typed (`#[source]`, `#[from]` or `#[error(transparent)]`) and leave it out of the
message: print errors with their chain (`error: cause: cause`).

## 0.9.0

Every BDD feature proves a success and a refusal.

**`bdd-feature`** is one `@succeeds` and one `@refuses` scenario; the hollow "the crate
composes its services" scenario is gone. Its steps are the author's, so a freshly
generated crate's BDD test fails until its behaviour is written. A `@refuses` scenario
drives the crate's public behaviour with invalid input and asserts the typed refusal;
scenario names use the case vocabulary (U1): succeeds, refuses, fails, rejects.

**`bdd-runner` (template)** declares only the world, `struct Behavior` deriving
`cucumber::World`, and `main`; no steps, so none are required of every crate. `main`
binds the adjacent feature once, parses it with cucumber's own Gherkin parser
(`::cucumber::gherkin`, re-exported by cucumber, no new dependency), asserts that a
scenario, directly or under a `Rule:`, is tagged `@succeeds` and one `@refuses`, then runs
exactly that feature with cucumber's default command line (`with_default_cli()`, so no
`--input`, `--name` or `--tags` can run another feature or a subset):

```rust
let feature = /* 0.8.0's Cargo-metadata path to the adjacent feature */;
let cases =::cucumber::gherkin::Feature::parse_path(&feature,::cucumber::gherkin::GherkinEnv::default());
for case in ["succeeds", "refuses"] {
 let tagged = cases.as_ref().is_ok_and(|parsed| parsed.scenarios.iter().chain(parsed.rules.iter().flat_map(|rule| rule.scenarios.iter())).any(|scenario| scenario.tags.iter().any(|tag| tag == case)));
::std::assert!(tagged, "the adjacent feature parses and has a @{case} scenario");
}
let result = <Behavior as::cucumber::World>::cucumber().with_default_cli().fail_on_skipped().run(feature).await;
```

`main` is the fallible shell (`Result<(), cargo_metadata::Error>`, `exec()?`), needing no
`expect`. The template renders no crate name, so it has no params.

**`bdd-runner` (rule)** holds `main` to that shell node for node, in its rustfmt form (the
rest is 0.8.0's: both mains, the formatting it ignores, the one file-level
`#![expect(..)]`), so the check cannot be weakened, skipped, moved after the run or pointed
at another feature. It also refuses, in a step (a function under an attribute whose path
ends in `given`, `when` or `then`): a `regex = ".."` match (use a cucumber expression or a
plain string), and a first parameter not named `world`; and anywhere in the runner, a
declared `Observation`, the retired scenario's state. A raw identifier counts as its plain
spelling. Each finding names what it found. What it cannot see: whether a `@refuses`
scenario really reaches a typed refusal (the tag is the author's claim), and the Gherkin
itself, which no provider parses, so the tags are asserted when the
runner runs.

Moving up from 0.8.0: in `main`, bind `run`'s argument as `let feature =..;`, add the tag
check before the run, call `.with_default_cli()` on `cucumber()` and `run(feature)` (the
template shows the whole shell); name each step's world parameter `world`; turn regex
steps into cucumber expressions; delete `Observation`, the composes-its-services steps
and scenario; tag each scenario with its case, adding a real `@refuses` (or `@succeeds`)
scenario where the feature has none.

## 0.8.0

Places `crate-root` 0.4.0: a macro crate's root no longer denies
`clippy::disallowed_types`. Our own checks are Tastemaker rules, never clippy configuration. Everything else is 0.7.0's.

Moving up from 0.7.0: delete `clippy::disallowed_types` from a macro crate root's two
`#[deny]` attributes.

## 0.7.0

`bdd-runner` rule 0.2.0: the runner may carry exactly one
file-level `#![expect(..)]` with `reason = ".."`, stating the lint expectations its steps
share once instead of on every step. Everything else is 0.6.0's (cpu-time, `cargo-package` 0.4.0, `crate-bench` 0.4.0).

Moving up from 0.6.0: nothing is required; a runner may collapse its repeated
per-step `#[expect(..)]` into one `#![expect(..)]` at its top.

## 0.6.0

No `nix`: the manifest is `cargo-package` 0.4.0, which dev-depends on
`cpu-time`, and the bench is `crate-bench` 0.4.0, which reads this process's CPU time
through `cpu_time::ProcessTime`.

Moving up from 0.5.0 (whose S2 split templates and `end-to-end` 0.3.0 this release keeps): follow `cargo-package` 0.4.0's and `crate-bench` 0.4.0's
changelogs.

## 0.5.0

The first spec placing S2's split shapes. A
long `tests/end_to_end.rs` splits by feature into `tests/<feature>/end_to_end.rs`
(`end-to-end-feature`), one directory per feature holding that one file; what several
features share lands in `tests/common/<fixture>.rs` (`end-to-end-common`). A long
`benches/performance.rs` splits by workload into `benches/performance/<workload>/bench.rs`
(`bench-workload`), with what several workloads share in
`benches/performance/common/<harness>.rs` (`bench-common`). Both are optional: a crate
whose tests or bench fit in one file keeps them whole.

The split files stay modules of the crate's one end-to-end test target and one bench
target. `tests/end_to_end.rs` (`end-to-end` 0.3.0) and `benches/performance.rs`
(`crate-bench` 0.3.0, still held to CPU budgets) remain their roots, still required, and
declare every split file by a `#[path]` module that their templates render from the
captures, so a split file no root declares is a template finding rather than a test that
never runs. One target keeps the shared fixtures compiled once and every fixture used, and
the manifest's targets are unchanged (`cargo-package` 0.3.0).

Moving up from 0.4.0: change the entry's `version` to `"0.5.0"`; the params are unchanged.
A crate that does not split needs no other change.

## 0.4.0

One README format. The crate's `README.md` follows
`crate-readme` 0.1.0, compared closed and in order: `# <package>`, the purpose on one
line, `## Usage` with one `rust` example, and `## Domains` with one `### <domain>` per
hex instance in the crate, each described on one line. `crate-root` 0.3.0 makes the README
the crate's documentation, so the example
runs as a doctest. `end-to-end` 0.2.0 runs the product README's Usage from a command line
crate's end-to-end test.

Moving up from 0.3.0 (whose `crate-bench` 0.2.0 and `cargo-package` 0.3.0 this release keeps): rewrite the README in the format, replace the root's `//!` comment
with the attribute, and, in the CLI crate, add `trycmd` and the `readme` test.

## 0.3.0

Every crate bench holds a CPU budget: the bench is `crate-bench` 0.2.0,
whose `budgeted` holds each workload's median run CPU time to a budget in a release
build, and the manifest is `cargo-package` 0.3.0, which dev-depends on `nix` for it.

Moving up from 0.2.0: follow `crate-bench` 0.2.0's and `cargo-package` 0.3.0's
changelogs.

## 0.2.0

The package is named for its product and folder. The spec declares two new
params, `product` and `folder`, which `cargo-package` 0.2.0 renders into `[package] name`:
`<product>-<folder>` for a folder's main crate, `<product>-<folder>-macros` for its macro
crate, `<product>-<folder>-<crate>` for any other, and `<product>` for the product's own
crate, which passes the product as its `folder` and `crate`. The product's root
`tastemaker.toml` passes `product` and captures `{folder}`. `crate-root` 0.2.0 declares a
macro crate's `composition` and `macros` modules under the macro spec's lint levels.

Moving up from 0.1.0: pass `product` and `folder` on the entry and rename each package.

## 0.1.0

The first v2 release. It places the package files, `src/lib.rs`, `src/composition.rs`, the BDD feature and runner, the end-to-end test and `benches/performance.rs`, and applies `bdd-runner` to `tests/bdd.rs`. A crate's domains are placed beside it by the `hex` and `domain-module` entries of the `tastemaker.toml` that places the crate. no-json-in-process is a clippy lint (the `clippy-config` template), not a rule of this spec.

## Rule help that says how to act

Every finding is printed with its rule's help, so the help of each rule this spec owns
says what to change and where, and whether an ignore with a reason is ever legitimate,
with the exact ignore when it is. The rules' text states its reasons itself and cites no
design ids.

**.** The adapter-crate branch is gone: `tested-by-cases` selects only a core's, an SDK's and a command line's files, and the library root and README no longer list a crate's hexes from captured adapters.

A core's library root declares neither `fixtures` nor `collections`; the contract is `fixtures/contract.rs` of its port.

**.** An SDK is detected by `src/resources.rs`, and the library root declares `resources` for a command line and an SDK.

- The `conformance` feature is deleted; a core's test support is compiled under `fixtures` only.

**.** The spec and its
`cargo-package` template take three more required params, each `absent` unless the crate sets it:
`package` (the Cargo package name, instead of the one `product`/`folder`/`crate` give), `library`
(a `[lib] name`) and `binary` (a `[[bin]] name` at `src/main.rs`). A product whose package names are
not its directory names states them in its
`tastemaker.toml` and the manifest is generated, not hand-edited. Every `crate` entry states all three
(`absent` keeps the old names). New fixtures `named-library`, `named-binary`, `library-missing`.

The `cases` template's `Expect<V, E>` is `Value(V)` or `Error(E)`: a row states the full error value and `run` compares it with `==` (`PartialEq` + `Debug`); there is no selector row (`Refusal`, `Selection`, `selects`). Edited in place: the package is frozen.

The system allocator: `cargo-package` no longer lists `mimalloc.workspace = true`; a manifest may still carry it (the template accepts extra content).

**The `kernel` param**. A kernel crate (the values crate and the language crate, the product's shared vocabulary) renders `pub mod domain;` in its `src/lib.rs`; every other crate renders the private `mod domain;`. The `kernel` param is `kernel` or `absent` and required, so each entry states it; the `crate-root` template takes it. Fixtures: the `nu-hex` spec's `kernel-complete` and `kernel-private-domain` show both ways (the template alone sees no `present`, so its own fixtures only pass the param).

**A kernel crate keeps the test support and needs no `cpu-time`**. The `cases` condition (rule `tested-by-cases`) also holds for `src/domain.rs`, so a kernel crate (the values crate: a domain and no use case) places `src/cases.rs` and a crate with a domain and no test support is a finding (fixtures `kernel-domain-cases`, `domain-without-cases`). The `cargo-package` template takes the `kernel` param and lists the `cpu-time` dev-dependency only when it is `absent`: the budgeted workloads that read the CPU clock are an ordinary crate's, and a kernel crate's bench declares none (template fixtures `kernel-without-cpu-time`, `ordinary-without-cpu-time`).

**`crate-root` renders `pub mod domain;` for a kernel crate without an inbound module**. The domain declaration moved out of the `present.inbound` branch: a kernel crate (`kernel = "kernel"`) with a domain list renders `pub mod domain;` with or without `inbound`; every other crate still renders the private `mod domain;` only beside `inbound`. The template's fixtures are placed alone and see no `present`, so the cases are the nu-hex fixtures `kernel-without-inbound` and `kernel-domain-undeclared`.

**The `kernel` param is gone, and nothing replaces it** `crate` takes no `kernel`. The library root declares `mod inbound;`, `mod outbound;` and `mod domain;` without a visibility, so one root serves a feature crate (`pub mod inbound;`, a private domain), a language provider (`pub(crate) mod inbound;`) and a kernel crate (`pub mod domain;`); a template that names a visibility is compared exactly and one that names none accepts any. The `cpu-time` dev-dependency is no longer asked of the manifest (a kernel crate has none, and a template cannot vary by role). `bdd-members` also reads `src/inbound/contracts/*.rs`, so a provider's features are named after its contracts. The fixture `kernel-domain-cases` is `domain-with-cases`; `without-cpu-time` is gone. A condition cannot choose between two templates for one file: a count of 0 means the file must be absent.

**The bench is present exactly when the crate has a workload**. `crate-bench` takes `count = { min = 0 }`, and `cargo-package` renders the `divan` dev-dependency and the `performance` bench target only when `benches/performance.rs` exists (`present["crate-bench"]`), so a crate with no workload has neither the file nor the target and an empty bench is never written. Fixtures: `missing-bench` is now `no-bench` (passes) and the new `bench-without-target` (a bench file the manifest has no target for) is the finding.

**The crate README's Usage opens with a line**. Under `## Usage` the README holds one paragraph on how a program uses the crate, then the example (`crate-readme`); a language provider's README, whose usage is a function the linking program calls, says so before the example. Every README the fixtures carry has the line.

**A language placed beside its crate scaffolds a crate that compiles.** The `cargo-package`, `crate-root`, `crate-bench` and `fixture-tree` templates read `present["language-conformance-test"]` and `present["language-fixtures"]`, the entries the `language` spec places at the crate's directory. Beside them the manifest depends on the SDK (`ryo`), declares the `allocation` and `conformance` test targets without the libtest harness, takes the SDK's dev-dependency with its `fixtures` feature, the crate itself as a dev-dependency with `fixtures`, `dhat` (and its `cargo-machete` note), and its `fixtures` feature forwards to the SDK's. The root keeps the `cfg_attr` that lets the ports' fixtures go unused outside tests and holds `linked`, which composes the `parse` and `project` contracts over the factory of the syntax adapter named for the language (`ryo::link!`, the name being the directory's); the bench holds the language's workloads (`generated`, `read`, `timed`, `complexity` against `GROWTH_LIMIT`, and a `#[divan::bench]` over N and 4N); a `relations/<relation>/expected.rs` of the fixture tree holds no row until the language reads one. Every other crate renders as before. Fixtures (in the `language` spec, because the entries live there): `crate-of-a-language`, `crate-without-test-targets`, `crate-without-linked`, `bench-without-generated`, `fixture-expected-without-rows`.

`fixture-tree` also leaves out `tests/fixtures/standard/**` and `tests/fixtures/relations/**`: the language spec places the pinned standard, and the input and the expected rows of each relation fixture (paired, so the input that the workspace keeps rules off is still counted). Fixture `fixture-held-by-language`.

**An SDK's root declares `mod resources` with a visibility of its own.** The `crate-root` template writes the module without one, so the SDK that names each resource at its root (`pub use resources::{..}`, `ryo::query`) keeps the module private and no item gains a second path `ryo::resources::…`.

**A language crate depends on the SDK as `idiomatic-sh-ryo-sdk`**: the `cargo-package` template's `language-conformance-test` dependency names that package (`ryo = { package = "idiomatic-sh-ryo-sdk", ... }`, library still `ryo`), the command line took `idiomatic-sh-ryo`.

- **No budgets, no divan requirement.** `crate-bench` asks for a `fn main` and, for a split bench, the declared workload modules; it keeps no `budgeted` helper, and `cargo-package` no longer requires `divan.workspace`. `bench-workload` is only the workload's doc. A product's measured workloads are the command line's `commands` bench (the `cli` spec).
- **The growth check times the fastest of five reads.** `crate-bench`'s `timed` returns the fastest of `ROUNDS` reads of each size, so one disturbed read never fails the check; `GROWTH_LIMIT` is unchanged.
