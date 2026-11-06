# cli

## 0.12.1

`factory-calls` leaves the generated `src/languages.rs` out: it is the one edge that links the language providers, built by `build.rs` from the directories of `languages/`, and hands the parts of each language to the language factory as a composition does. The scenario `languages-edge-names-the-factory` holds it; every other file of the command line's own that names a factory is still a finding (`infrastructure-calls-factory`). The `languages` template and the generated file are unchanged.

### Moving up from 0.12.0

- Nothing to change: the rule only stops reporting the generated file.

## 0.12.0

Edited in place: **the command tree's root builds the `Edge` and passes it** (`commands`, `cli-main`). `commands` rendered `{{resource}}::run(command)`, while `resource` takes `run(command, edge: &crate::edge::Edge)`, so a scaffolded command line did not hand its commands the edge the three products hand theirs. The root now flattens `crate::output::FormatSwitch` and `crate::output::dry_run::DryRunSwitch`, makes `--format`, `--json`, `-q` and `-n/--dry-run` global in `command()`, takes `run(start: &str, build: &str)`, builds the `Edge` once in `run_from` and dispatches `{{resource}}::run(command, &edge)`; `cli-main` reads the process's directory and calls `<product>::resources::run(&start, env!("CARGO_PKG_VERSION"))`. No item is added to either template, so the products, which already do this, are unchanged. Every scenario's `resources.rs` and `main.rs` here shows the edge built and passed.

Edited in place: **`writing` only where a command writes** (`edge`). The `edge` entry passes the spec's `writes` to the template; when `writes` is empty the command line takes no `-n/--dry-run`, so the rendered edge has no `writing` field (five fields) and its doc comments leave the dry-run switch out. Scenario `rendered-template-without-writes-passes` renders it with `writes = []`. Ryo, whose `writes` is empty, drops `writing`; asymptote and Tastemaker keep it. The `commands` entry passes `writes` the same way: when it is empty the root flattens no `DryRunSwitch`, makes only `--format`, `--json` and `-q` global and builds the `Edge` without `writing`; scenario `rendered-template-without-writes-passes` renders it with `writes = []`.

Edited in place: **`main` runs on the allocator ruff and uv ship** (`cli-main`): `#[global_allocator]` is `tikv_jemallocator::Jemalloc` where jemalloc builds (not Windows, OpenBSD, AIX or Android; x86_64, aarch64, powerpc64 or riscv64) and `mimalloc::MiMalloc` on Windows, under ruff's own `cfg`. The system allocator kept what each worker thread freed in that thread's arena, and runs were slower; on ryo's ruff bench jemalloc cuts the warm run by about a quarter and an edit by 5-10%. The cold peak is about 60 MB higher, because jemalloc keeps freed pages for a while. The workspace pins `tikv-jemallocator` (`=0.6.1`, `background_threads_runtime_support`) and `mimalloc` (`=0.1.52`, `v2`) as ruff does, and the command line crate depends on each under the same target condition. Scenario `system-allocator` proves that a main without the allocator statics is a `cli-main#template` finding. Every scenario's `main.rs` here and in `crate` 0.13.0 carries the statics.

### Moving to the allocator

Write the two statics into `cli/src/main.rs` as the template renders them, add `tikv-jemallocator` and `mimalloc` to `[workspace.dependencies]`, and add the two target-conditioned dependency tables to the command line crate's manifest.

Edited in place: **one `Edge` for every command** (`edge`, `src/edge.rs`, required). It holds what every command reads of where it runs: `start` (where the project is looked for), `here` (the directory the command line was started in), `store` (the directory beneath a project's root where the store is kept), `build` (the build that runs), `format` (`--format`, `--json`, `-q`) and `writing` (`-n/--dry-run`). `main` reads the process's directory and build; the command tree's root reads the switches, which are the command line's, global, and builds the `Edge` once; every command is `run(arguments, edge: &crate::edge::Edge) -> Outcome` and its arguments hold its own flags only. The template's content is Tastemaker's edge; a field is a part of its struct, which the comparison does not read, so each product writes its own types in the six fields. `resource` dispatches `run(arguments, edge)`; the `command` spec's `command` template takes the edge and its `imports` allow `crate::edge`; `crate-root` declares `pub mod edge;`. Scenario `command-reads-the-current-directory` proves a command that reads `std::env::current_dir` itself is an `io-at-boundary` finding, the rule the product places beneath `main.rs`; every scenario's crate holds `src/edge.rs`.

### Moving

Move the product's edge into `src/edge.rs` as the template renders it, with the product's own field types; declare `pub mod edge;`; make `--format`, `--json`, `-q` and `-n/--dry-run` global on the command tree's root and build the `Edge` there once; give every command `run(arguments, edge: &crate::edge::Edge)` and read the format and the writing from it.

Edited in place: no file of a command line lies on a ring of module dependencies (`module-cycles`). `--quiet`'s `QUIET` is `output-progress`'s, which reads it, and `output`'s `quiet` and `output-dry-run`'s `planned` name it there; `timings-phases`' `install` answers its report, which `cli-timings` keeps in its guard, so it names nothing of `timings.rs`. The command tree's `Edge` is the crate root's (`crate::Edge`), and a command's `schemas.rs` takes the values of its arguments, never the command's clap struct, so `command.rs` names its schemas and not the reverse.

Places `command` 0.2.0, and through it the `composition` template 0.2.0, whose `compose` is `pub(crate)` (rustc's `unreachable_pub`); nothing else changes. The scenarios place `cli` 0.12.0 and their compositions are `pub(crate)`. A product moves by changing the `version` of its `cli` placement to 0.12.0 and writing each command's `pub(crate) fn compose`.

Scenario `validation-at-edges-skips-a-command-part` proves the own-file `validation-at-edges` rule leaves a command part's `schemas.rs` to the `command` spec.

## 0.11.0

The command line crate holds its `LICENSE` (`license`, `count = 1`): the MIT license text, a file of its own, never a link
to one outside the crate. The distribution's smoke tests stage it from the crate's directory and the npm, PyPI and
crates.io packages ship it, so a newly placed command line carries it from the start. Every fixture holds the file; the
template's `file-missing` fixture is the finding.

**The panic report is said through `output`** (`output::panicked`): `report_panics()` in `resources.rs` installs it and writes nothing itself, so `resources.rs` no longer imports `std::io::Write`. Only `output` and the timings report name a standard stream (`stdio-at-presentation`, idiomatic-rust).

### Moving up from 0.10.0

Write `cli/LICENSE` as the template renders it (replace a symbolic link with the file itself).

## 0.10.0

The command line's lock: connections, schemas and one test shape, edited in place.

- **`connections.rs` beside `composition.rs`** in each command (`command-connections`): it opens the stores and returns their clients, the only I/O of the edge. `composition.rs` takes the clients and the parts and only constructs (`compose(connections, parts)`; `build()` calls `connections::open()`), and `composition_test.rs` runs the real wiring over fake connections. `command-composition`, `command-module` (it declares `connections` and `schemas`), the shared `composition-test` and the description of the shared `composition-parts` follow. The product's boundary rules (`io-at-boundary`, `threads-at-adapter`, `no-json-in-process`, `adapter-at-root`) exclude `**/composition.rs` today and must exclude `**/connections.rs` too.
- **`schemas.rs` and `schemas_test.rs` at each command** (the shared `schemas` and `schemas-test`, `count = { min = 1 }`): the shape of the arguments the command is given and their checked conversion into the core's types; the command's `run` converts through them (`arguments.try_into()`, a refusal is `Outcome::Failure`). `validation-at-edges` is placed over `src/**` but the schemas and the tests: a `TryFrom`, an `impl FromStr` or a fallible constructor anywhere else in the crate is a finding.
- **`command-test` is the one test shape**: a `CASES` table (rows of a name, one case word, an argv and the printed answer or the outcome) and the four template-owned runners `succeeds`, `refuses`, `fails` and `rejects` calling `crate::cases::run`, with the three aliases `Input`, `Value` and `Failure` standing for the table's types (the comparison pins a constant's type, so the aliases keep it free). It no longer writes `<command>_succeeds` asserting only `Outcome::Success`, which passes whatever the command prints: the subject is `todo!` until a row states the command line and the answer it prints. `test-shape`, `case-expectation` and `unit-cases` are placed over `src/commands/**/*_test.rs`; `case-vocabulary` is not, because it reports the bare `fails` of the runner (it allows `fails` only as `fails_when_<dependency>_fails`).
- **`command-refuses` reads rows**: a command owes a `refuses` row, the identifier `Refuses` inside the constant `CASES` (the runner `fn refuses` and a helper outside the table do not count). The flatten and `value_enum` exemption and the ignore are unchanged. The compiler-resolved `command_contract` lint in `tools/lints` still names `<command>_preserves_supplied_service_contract` and `<command>_fails_when_the_service_fails`; it is re-pointed to the `fails` rows.
- **`readme-test`** (`tests/readme.rs`): the product README's Usage run against the built binary, which the crate spec's `end-to-end` target ran before that target left the crate spec.
- The imports allow-lists drop `*::value` (there is no value split) and the `command` entry may import its `schemas`. The description states the principle of one alphabet of primitives, each file one of them.
- **The command line is flat or nested; the code stays grouped by resource.** Commands live at `src/resources/{resource}/{command}` either way. The tree is typed `<command>` (flat) or `<resource> <command>` (nested), and the generated `command-tree-contract` holds it to one of the two: every command is a verb with no subcommand of its own, and the root's commands are either all verbs or all resources, never a mix (the case `every_resource_holds_commands_and_no_command_holds_more` became `every_command_is_a_verb_with_no_subcommand`). `command-line-contract` walks either tree. `commands` declares what both forms share, so it no longer pins `name`, `version` or `about` in `#[command(...)]` (the package names the binary, `command()` sets the version, and the contract tests hold both) and declares the `Resource` enum only for a tree of two or more resources; a tree of one resource generates the flat form. `writes` names each command as typed. Fixtures: `flat-tree` passes beside `complete` (nested); `command-tree-contract/fixtures/flat-tree` passes; the `no-version` fixture of `commands` is gone with the pin it proved.
- **`cli-allocation-test` and `cli-rule-bench` are `count = { min = 0 }`**: a command line that counts blocks or runs rules has them; one that does not holds no placeholder.

Moving up from the previous edit: add `connections.rs` and `schemas.rs` with its `schemas_test.rs` to each command and declare them in its module file; give `composition.rs` the `compose(connections, parts)` shape; rewrite each `command_test.rs` as a table and the four runners and each `composition_test.rs` likewise; add `tests/readme.rs`. Fixtures: `complete` and the rest follow; passing `complete`; failing `missing-connections`, `missing-schemas`, `missing-readme-test`, `validation-outside-schemas`, `schemas-test-without-a-success`, `missing-runner` (it replaces `missing-service-contract`) and `extra-command-test` (it replaces `misnamed-command-case`).

The imports allow-list names `*::value` beside `*::types`: a value with behaviour is
`types/<value>/value.rs` of its hex.

A test beside every placed logic file:
`src/outcome_test.rs`, `src/outcome/remedy_test.rs`, `src/output_test.rs` and
`src/output/{dry_run,pager,progress}_test.rs` are placed with `count = 1` (`outcome-test`,
`outcome-remedy-test`, `output-test`, `output-dry-run-test`, `output-pager-test`,
`output-progress-test`), and each logic template declares its test as `timings` does,
`#[cfg(test)] #[path = "<file>_test.rs"] mod tests;`. Fixture: failing
`missing-pager-test`.

Every command line is held to the Command Line Interface Guidelines (clig.dev) by what the
spec generates, so no one reads clig.dev again:

- `commands`: the root takes `#[command(name = "<product>", version, about)]` and
 `#[command(propagate_version = true, after_help = SUPPORT, arg = no_color())]`, so
 `--version` and `-V` answer on every command, the top-level help says where to report a
 problem, and `--no-color` is global; `command()` returns the tree; `run` installs
 `report_panics()` and catches a panic, which ends as exit 2; `run_from` applies
 `color(arguments)` before clap prints anything and parses with `try_parse_from`, so help
 and usage errors end through `usage(error)`, where a closed pipe is not a failure. These
 belong to the command line itself, so they sit here, private; the template takes the
 `product` param. `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE` and `TERM` stay anstream's and
 clap's to read (`no-env-reads` holds: our code reads none).
- `output`: `FormatSwitch` adds `--json` and `-q/--quiet` (a `Quiet` format that prints
 nothing); `present` treats a closed pipe as the reader's choice, not a failure; `fail`
 prints `error: <message>` on stderr in every format.
- New closed templates `command-tree-contract` (`tests/command_tree.rs`) and
 `command-line-contract` (`tests/command_line.rs`), each a test target Cargo finds on its
 own, generated whole, with
 `product` and the new required spec param `library` (the command line's library crate):
 over the tree, two levels, help text everywhere, no alias, catch-all or prefix sibling
 (a name and its plural excepted), a long for every flag and only the standard shorts
 (`-h -V -q -o -n -f`), one positional at most, global root flags, `--format`, `--json`,
 `--quiet` and `--no-color` on every command, and an `Examples:` block in every command's
 help; over the binary, `--help`, `-h` and `help` on every path, help on a bare command
 (exit 2), the version everywhere, an unknown flag as `error:` on stderr with exit 2, a
 mistyped resource suggested and an abbreviated one refused, the support line, and styled
 help that `NO_COLOR` and `--no-color` turn off. `closed` compares declarations, not
 bodies, so an assertion weakened in place is not yet seen.

Fixtures: `complete` carries both contract tests and the new `commands` and `output`; new
failing `missing-contract-test`; `commands` gains failing `no-version`, `no-command-fn`,
`no-support-line`, `no-colour-switch` and `no-panic-report`; each
contract template has `example`, `extra-content`, `case-switched-off` and `missing`. A
struct's fields are not part of a template's contract (only items are), so `--json` and
`--quiet` are proven by the contract tests, not by an `output` fixture.

The clig.dev changes:

- **Every error names a next step** (Errors, Output): new template `outcome-remedy`
 (`src/outcome/remedy.rs`) declares `Next` (`Run`, `Fix`, `Report`) and `trait Remedy`,
 with `#![deny(clippy::wildcard_enum_match_arm)]`, which the template contract compares, so
 every `Remedy` impl a product writes there names every variant and a new error variant
 does not compile until it is given a next step; `output::fail` takes no error without a
 `Remedy` and prints `error: <cause chain>` then `help: <next>` (JSON gains `help`);
 `outcome` declares `pub mod remedy;` and keeps the deny for its own matches. An answer
 stdout refuses is said on stderr with its next step (`unwritten`), never silently.
 Failing fixtures `outcome/wildcard-allowed`, `outcome/no-remedy-module`,
 `outcome-remedy/no-next`.
- **Progress on stderr** (Robustness): new template `output-progress`
 (`src/output/progress.rs`): `layer()` is a tracing layer drawing each event with target
 `progress` as one indicatif line, only when stderr is a terminal and `--quiet` was not
 asked (`output::quiet`, read before parsing); `cli-timings` adds it to the subscriber it
 installs, and `settle()` takes the line down before any answer, error or planned change
 (the next event draws it afresh). Failing fixtures `output/no-progress-module`,
 `output-progress/no-settle`.
- **`-n/--dry-run` on every command that writes** (Arguments and flags): new template
 `output-dry-run` (`src/output/dry_run.rs`) declares `DryRunSwitch` and `planned`
 (`would write <path>` on stderr, silent under `-q`); the new required spec param `writes`
 lists the `<resource> <command>`s that write, and `command-tree-contract` holds that each
 takes `--dry-run` and no other command does. A command composes a planned adapter over
 each port that writes, which reports each change through `planned` and makes none.
 Failing fixture `output-dry-run/no-planned`.
- **A pager** (Output): new template `output-pager` (`src/output/pager.rs`): terminal text
 goes through `PAGER`, or `less -FRX`, when stdout is a terminal, as git, gh and jj page;
 `PAGER` is `no-env-reads`' named exception `pager`, and a
 product's `io-at-boundary` entry excludes this one file. Failing fixture
 `output-pager/no-pager-choice`.
- **The command surface** (Future-proofing): new template `command-surface` places
 `tests/command_tree/surface.txt`, which `command-tree-contract` renders from the tree
 (each command, its flags with shorts and value names, its positional by value name) and
 compares through snapbox (`SNAPSHOTS=overwrite` rewrites it); a change to the surface is
 a visible change to that file. The rule that a removed line needs a deprecated line
 waits until the command surface is stable.

### Moving up within 0.10.0

Add `library` and `writes` to the `cli` entry's params, rewrite `src/commands.rs` and the switch in
`src/output.rs` as the templates render them, scaffold the two contract tests, then make the command line pass them: an
`Examples:` `after_help` on every command's arguments, `-o` on `--output`, `-n` on
`--dry-run`, and every second positional turned into a flag.

Composition uses scoped typed `Parts`, `compose`, and production `build`.
The active `composition-wiring` rule rejects production selection in wiring;
the required native construction lint resolves aliases and macro-expanded calls.
Optional `types.rs` and `composition_test.rs` owners separate dependency values and
consequential substitution proofs from the wiring. Intermediate and returned services
use supplied constructor callbacks; settings and shared instances flow through them.
The command still receives its completed service, with no new user-facing options.

The CLI owns `cli-timings` and its deterministic `timings-test` contract. Commands
flatten `TimingsArgs` and keep the report guard until command completion. Wall
attribution excludes overlapping active children once; external distribution timings
remain the authority for whole-process wall and CPU. The working crate manifest and
workspace scaffolds provide tracing, tracing-subscriber and tracing-chrome.

A product generated from the `devtool` starter compiles on scaffolding alone. The command line's package is named after its product (the `crate`
spec's `cli-crate` entry), so the spec takes a required `product` param and passes it to
`cli-main`, whose `src/main.rs` calls `<product>::commands::run()` (0.9.0 called the
directory's name, `cli::commands::run()`, which no product's package is), and to
`command-bench`, whose bench calls `<product>::commands::...` and parses its arguments
with clap rather than naming their private fields. `commands`' `run` is `#[must_use]`,
which clippy's pedantic group asks of it. Everything else is 0.9.0's.

Fixtures: the cli-main and command-bench fixtures pass `product`; `complete`'s manifest
depends on `anstream`, `clap`, `mimalloc`, `serde`, `serde_json` and `trycmd` (`crate`
0.12.0's `cargo-package`); new failing `no-product`, `cli-without-dependencies` and
`output-undeclared`.

A command line crate has no BDD files: `complete`'s
command line lost `tests/bdd.feature`, `tests/bdd.rs`, its `[[test]] name = "bdd"` target and
its `cucumber` dev-dependency, as did the manifests of `cli-without-dependencies` and
`unregistered-command-bench`.

`command-refuses` owes no `refuses` case when every field of the struct the command's
`run` takes is `#[command(flatten)]` or `#[arg(value_enum)]`: clap refuses those before
`run`, so the command has no refusal of its own. Only that struct
decides, and `flatten` and `value_enum` count only as option names of their own attribute.
The rule reads the `command.rs` beside the test file, so its entry includes
`src/commands/**/command.rs`; a finding is still about `command_test.rs`. Rule fixtures:
new passing `parsed-arguments` and `other-struct` (another struct's plain field), new
failing `free-argument` (one plain field among flatten fields), `option-value`
(`help = flatten`, `default_value = value_enum`), `foreign-attribute` (`#[serde(flatten)]`)
and `own-plain-field` (only another struct is fully parsed). Spec fixtures:
`complete`'s `greeting list` drops its ignore, and `missing-refusal`'s `greeting show`
takes a `name`.

A command line that links language providers holds their links in one edge file
(they are deliberately not in the
crates/ and sit in languages/): new template `languages` at `src/languages.rs`, optional,
present exactly when the command line links a provider, and `crate` 0.12.0's `crate-root`
then declares `mod languages;`. The file holds one `#[cfg(feature = "<language>")] use
<provider> as _;` line per provider, which are the product's own. Not enforced here: that
no other file of the command line links a provider. A link is `use <crate> as _;`, which
the syntax cannot tell from a trait import without naming the product's provider crates,
and a shared rule takes no path or glob param; which crates may depend on a provider is
the product's cargo-deny `[bans]`. Fixtures: new passing `linked-languages`, new failing
`languages-undeclared`; the `languages` template has `example`, `extra-content` and
`missing`.

Each command's `composition.rs` is held to the shared `hollow-services` 0.1.0: a `Services` there holds a service or is absent. New failing
`hollow-services`. `no-root-composition`'s reason follows `crate` 0.12.0, which places no
`src/composition.rs` in any crate.

### Moving up from 0.9.0

`tm workspace update` takes `cli` 0.10.0 and `crate` 0.12.0. A command line that
links language providers moves their `use <provider> as _;` lines into `src/languages.rs`
and declares `mod languages;` in `src/lib.rs`. Give the `cli` entry
`product = "<product>"` in its `params`. `src/main.rs` calls `<product>::commands::run()`;
each command bench calls `<product>::commands::<resource>::<command>::command::run`;
`src/commands.rs`' `run` gains `#[must_use]`. The CLI's `Cargo.toml` follows `crate`
0.12.0's changelog.

## 0.9.0

Every answer is printed by one small module, and a command never knows the format. The per-command shape files of
0.6.0 are gone:

- New template `output` at `src/output.rs`, one per command line: `Format` (a
 `clap::ValueEnum`), the `FormatSwitch` every command flattens into its arguments as
 `--format`, and `present`, which prints an answer that is `serde::Serialize` and
 `std::fmt::Display` and ends the command as the `Outcome` it is given, or as
 `Outcome::Error` when stdout refuses it: json is serde_json of the answer, terminal is
 its text written through anstream, which keeps anstyle styles where stdout takes them
 (a terminal, unless `NO_COLOR` or `CLICOLOR` says otherwise), and
 markdown, where offered, is its text with the styles dropped. A file may carry more
 than the template, such as how the product reports a failure.
- New spec param `markdown`, required: `offered` or `absent`. It is passed to `output`;
 `offered` adds the `Markdown` format and the module's `markdown` function, `absent` leaves
 both out, so a product with no markdown output holds no placeholder for it.
- The `command-terminal`, `command-json` and `command-markdown` templates and their entries
 are deleted, and `command-module` no longer declares `terminal`, `json` and `markdown`. A
 shape file left in a command directory is a `cli#closed` finding.
- `command` may import `crate::output`; its generated code flattens the switch and prints
 the answer through `crate::output::present`. `command-test`'s generated success case parses
 the command's arguments through clap, since the switch's field is private.
- An answer's text and JSON are the answer's own: its `Display` and `Serialize`
 implementations sit with the value, in the `types.rs` of the hex that owns it.
- Fixtures: `complete` has `src/output.rs` (markdown offered); new failing `missing-output`
 and `shape-file-left`; `missing-command-shape` is gone. The `output` template has passing
 `example`, `absent` and `extra-content`, and failing `missing`,
 `offered-without-markdown`, `no-switch` and `not-a-value-enum`.

The product rule `no-json-in-process` names `serde_json`: the entry that places it
excludes `**/src/output.rs`, where JSON crosses the process boundary.

### Moving up from 0.8.0

`tm workspace update` takes `cli` 0.9.0; give the entry `params = { markdown =
"offered" }` or `"absent"`, and `tm workspace scaffold` writes `src/output.rs`.
Declare `pub mod output;` in `src/lib.rs`, add `anstream`, `serde` and `serde_json` (and
`anstyle` where an answer is styled) to the crate's dependencies, and exclude
`**/src/output.rs` from `no-json-in-process`. For each command: implement `Display` and
`Serialize` on the value its service answers (or a value it builds from it), moving what
its `terminal.rs` and `json.rs` wrote there (and what `markdown.rs` wrote into the text,
when markdown is offered); replace the command's own format flag with `#[command(flatten)]
output: crate::output::FormatSwitch`; end with `crate::output::present(format, &answer,
outcome)`; and delete
`terminal.rs`, `json.rs`, `markdown.rs` and their `mod` lines.

### Moving up from 0.5.0

Skip 0.6.0's shape files: each command's rendering moves straight onto its answer value,
as in "Moving up from 0.8.0". Then, in order:

1. 0.7.0's `Outcome`: add `src/outcome.rs` (`outcome` template) and `pub mod outcome;`;
 every command's and resource's `run` returns `crate::outcome::Outcome`, converted to an
 exit code once at the tree's root; a product's own conversions into `Outcome` (from a
 verdict, from an error's class) move into `src/outcome.rs` beside the template's
 declarations. Each command test holds `<command>_succeeds`,
 `<command>_refuses_<what>` (or the reasoned `command-refuses` ignore) and
 `<command>_fails_when_the_service_fails`.
2. 0.8.0's composition: move what `cli/src/composition.rs` built for each command into
 that command's `composition.rs` (`build()` returning the service it calls), make `run`
 take only its arguments and call `super::composition::build()`, keep a seam that takes
 the built service so the command's tests run it against the port's fake, and delete
 `cli/src/composition.rs`.
3. This version's `src/output.rs`, as in "Moving up from 0.8.0"; a presentation hex the
 CLI kept (a `report/` or `presentation/` directory with an adapter per format) is
 deleted with the shape files.

## 0.8.0

Composition is owned at the edge, one per command. Each command directory gains `composition.rs`, the command's whole wiring: it
chooses implementations, builds each shared resource once and passes it to every factory
that needs it, and builds the service the command calls. `command.rs` calls
`super::composition::build()` once, so `run` takes only its arguments; `commands.rs`,
each resource module, `command_test.rs` and the command bench no longer pass a
`crate::composition::Services`.

- New template `command-composition` at `src/commands/{resource}/{command}/composition.rs`,
 one per command; `command-module` declares `pub(crate) mod composition;`.
- `command`'s imports allow its `command-composition` instead of `crate::composition`.
- The shared `factory-calls` 0.1.0 (only an edge names a factory) runs over `src/**` but
 each command's `composition.rs` and the tests; `no-runtime-fixtures` 0.1.0 over `src/**`
 but the tests.
- Fixtures: `complete` has a composition per command; new failing `missing-composition`,
 `command-calls-factory`.

`cli/src/composition.rs` is not the cli spec's: the crate spec places a crate's
`composition.rs`, optional from crate 0.11.0, and the CLI crate drops it.

### Moving up from 0.7.0

`tm workspace update` takes `cli` 0.8.0 (with `crate` 0.11.0); `tm workspace
scaffold` writes each command's `composition.rs`. Move what `cli/src/composition.rs` built
for each command into that command's `composition.rs` (`build()` returning the service it
calls), make `run` take only its arguments and call `super::composition::build()`, drop the
`services` argument from `commands.rs`, the resource modules, the command tests and benches,
and delete `cli/src/composition.rs` and its `mod composition;`.

## 0.7.0

Every command ends as an `Outcome`:
`Outcome { Success, Failure, Error }`, exit 0, 1 and 2: it did its job, it refused its
input or reached a negative verdict, or it could not run. The spec places and owns:

- `src/outcome.rs`, the `outcome` template: the CLI's own `Outcome`, `#[must_use]` so an
 ending dropped on the floor is a warning, with `code()` and
 `impl From<Outcome> for std::process::ExitCode`, and no `Termination` impl (`main`
 returns an `ExitCode`). Each CLI owns one; none is shared across products. A command may
 import `crate::outcome`. The template's declarations are the enum, `code` and the
 `ExitCode` conversion; the variant names are held by the generated `command` and
 `command-test` code, which names them.
- `command`, `resource` and `commands`: a command's and a resource's `run` return
 `crate::outcome::Outcome`, and the tree's root converts it to the exit code once. The
 generated command maps the service's answer to `Outcome::Success` and its error to
 `Outcome::Error`; a command whose service refuses input or reaches a negative verdict
 returns `Outcome::Failure`. `command-bench`'s generated workload asserts
 `<crate>::outcome::Outcome::Success` in place of `std::process::ExitCode::SUCCESS`.
- `command-test`: a command's tests declare `<command>_succeeds` and
 `<command>_fails_when_the_service_fails` (a failing service, composed through the
 command's own seam, makes `run` return the outcome the CLI gives a service error), each
 running the command and asserting the `Outcome` it returns. The generated `fails` case is
 a `todo!` naming the seam to fill.
- the `command-refuses` rule, over `src/commands/**/command_test.rs`: the file holds a test
 (a function with a `test` or `rstest` attribute) whose name has `_refuses_` in it,
 `<command>_refuses_<what>`, a name no template can fix. A command that refuses nothing
 records why with `// ignore: idiomatic.arch/cli/command-refuses - <reason>` at the top of
 its test file; the finding is about the whole file, so an ignore at the end of a line
 does not cover it. Read syntactically: the rule reads the test's name, not what it
 asserts, and does not check that the name starts with the command's name, since no
 relation gives a rule the command captured from the file's path.

That a command's `run` returns `Outcome` is `thin-command` 0.2.0, which the devtool
starter's shared rule block places at the product root, where it reads every
`interface.rs`.

Moving up from 0.6.0: add `src/outcome.rs` from the `outcome` template and declare it in
`src/lib.rs`; return `crate::outcome::Outcome` from every command's and resource's `run`
and from the presenters they call, converting to `ExitCode` only at the tree's root; make
each command bench that calls `run` assert `<crate>::outcome::Outcome::Success`; and give
each command the three tests, `<command>_succeeds`, `<command>_refuses_<what>` (or the
ignore, with its reason) and `<command>_fails_when_the_service_fails`, each running the
command against its services and asserting the `Outcome` it returns, not only parsing its
arguments.

## 0.6.0

Each command presents its answer in three shapes, one file each beside `command.rs`:

```
src/commands/<resource>/<command>/terminal.rs tab-separated lines (command-terminal)
src/commands/<resource>/<command>/json.rs a JSON document (command-json)
src/commands/<resource>/<command>/markdown.rs Markdown tables (command-markdown)
```

The owned `command-module` template now declares them. The spec places the files; what
each holds is the product's (in ryo, one trait per shape, all three required by the report
every command presents).

Moving up from 0.5.0: split each command's rendering into the three files, and declare
them in the command's module file.

## 0.5.0

A command's tests name their cases with the case vocabulary (,
): the spec places the `case-vocabulary` rule 0.2.0 over
`src/commands/**/*_test.rs`, as `hex` 0.6.0 does over a domain's tests (a command is not in
a hex). A test names at most one of `succeeds`, `refuses`, `fails` and `rejects`, and
`fails` only as `fails_when_<dependency>_fails`. The file set is unchanged from 0.4.0.

Moving up from 0.4.0: rename each command test the rule reports, keeping the one case word
it proves (case-vocabulary 0.2.0's CHANGELOG); behaviour is unchanged.

## 0.4.0

No `nix`: the bench root is `performance-main` 0.3.0, whose `cpu_time` reads
this process's CPU time through `cpu_time::ProcessTime`.

Moving up from 0.3.0: follow `performance-main` 0.3.0's changelog.

## 0.3.0

Every command bench holds a CPU budget: the bench root is
`performance-main` 0.2.0, which declares `budgeted`, and each command's bench is
`command-bench` 0.2.0, which runs its workload through `crate::budgeted` and fails unless
the command succeeds.

Moving up from 0.2.0: follow `performance-main` 0.2.0's and `command-bench` 0.2.0's
changelogs.

## 0.2.0

Each command's bench lives in the command's own directory (directories are the
domain concepts, files the design primitives):

```
benches/performance/<resource>/<command>.rs module file: `mod bench;` (command-bench-module)
benches/performance/<resource>/<command>/bench.rs the workload, measured with divan (command-bench)
```

The `command-performance` entry is replaced by `command-bench-module` and `command-bench`,
both 0.1.0; `command-bench`'s template is `command-performance` 0.1.0's. A bench left at
the old path is a template finding on its module file, and its `bench.rs` is missing.

Moving up from 0.1.0: move each `benches/performance/<resource>/<command>.rs` to
`benches/performance/<resource>/<command>/bench.rs`, and write `mod bench;` in its place.

## 0.1.0

The first v2 release: `src/main.rs` (declares the mimalloc global allocator), `src/commands.rs`, `src/commands/<resource>.rs`, `src/commands/<resource>/<command>.rs` with `command.rs` and `command_test.rs`, and the benches `benches/performance/main.rs`, `benches/performance/<resource>.rs` and `benches/performance/<resource>/<command>.rs`. A command may import only clap, std, the composition root and other crates' interfaces, types and errors. thin-command is a dylint lint. Supersedes the bootstrap's `cli-allocator` and `cli-performance` templates (now `cli-main` and `performance-main`).

## Rule help that says how to act

Every finding is printed with its rule's help, so the help of each rule this spec owns
says what to change and where, and whether an ignore with a reason is ever legitimate,
with the exact ignore when it is. The rules' text states its reasons itself and cites no
design ids.

Edited in place. A command is a part of kind `command`: the spec places the `commands` and resource lists and each command's module file, and the part spec holds the command, composition (`compose`), connections (`connect`), types and schemas files and their tests.

the `part` spec is retired for use-case, concept, port, item, adapter, command, entry and collection, and this version places the specs of its own parts, so a product's tastemaker.toml names only the crate and its crate kind.

**Heap boundary leaves the cli.** The cli no longer places `src/timings/heap/` (the `timings-heap-interface`, `-types`, `-composition`, `-files` and `-files-test` templates and the `heap-sink-module-missing` fixture are gone); that boundary is an adapter under a nu-hex port.

The heap diagnostic leaves the cli spec entirely: the `timings-heap`, `timings-heap-test` and `cli-main-test` templates, the `heap_diagnostic` param, the allocator switch in `main.rs` and `mod heap` in `timings.rs` are gone.

**.** Every command line has the phase observer: `src/timings/phases.rs` and `phases_test.rs` count 1 and `timings.rs` declares the module, beside the output, outcome with remedy, pager, dry-run and progress parts it already requires (clig.dev, every product). Fixtures: `missing-timings-phases` and `missing-timings-phases-test`.

**and.** The command tree lives at `src/resources.rs` (the `commands` template keeps its name, its file moves) with the verbs at `src/resources/<resource>/<command>/`; the library and the tests name `resources::`, not `commands::`. Benches mirror it as before.

- The `command-module` template declares `connections`, `composition_test` and `fixtures` when the verb holds them.

- **`tests/replay/{suite}.rs`** (`replay-suite`, `count = { min = 0 }`): one module of a product's replay test, open; a closed cli no longer leaves the replay test unplaced.

The `replay-suite` template moved into this spec: only `cli` places it, and a template only one spec names is the spec's own.

The system allocator: `cli-main` renders no allocator and its `no-allocator` check is gone; a main may still install one of its own (the template accepts extra content), so a product that keeps an allocator is untouched.
## Allocation test and per-rule bench

The command line crate's `tests/blocks.rs` and `benches/lint.rs` had no placement; the spec now owns them as the open templates `cli-allocation-test` (the `#[global_allocator]` static and `fn main`, run alone in its process) and `cli-rule-bench` (`fn main`, a process of its own for each rule), each `count = 1`. Their `[[test]]` and `[[bench]]` entries with `harness = false` are the crate's manifest, which the `crate` spec's cargo-package template already allows to carry more.

### Shared allocation test

The `cli-allocation-test` template moves out of this spec into the shared `allocation-test` template (`templates/allocation-test/0.1.0`), so the `nu-hex` spec places the same `dhat` test at `tests/allocation.rs`. The command line crate still has it at `tests/blocks.rs`; nothing else changes.

- **The per-rule test and bench follow the engine**: `tests/blocks.rs` (`cli-allocation-test`) and `benches/lint.rs` (`cli-rule-bench`) are present exactly when the command line depends directly on ryo's core engine crate, `idiomatic-sh-ryo-core` (the `links-engine` condition, answered by the owned `engine-linked` rule, which replaces `languages-linked`, from the crate's `Cargo.toml`), so a command line that composes the engine through the SDK and owns no rule has neither, and one that runs rules has both. Linking language providers no longer decides it. The `complete` fixture carries neither; `complete-with-rules`, `languages-without-rule-files` and `rule-files-without-languages` prove each direction.

### Global switches and optional positionals

The command-tree contract holds `-n/--dry-run` on every command that writes, its own or the command line's global one, so a command line may make the switch global (`tm -n check`); it no longer refuses it on a command that writes nothing (`every_command_that_writes_takes_dry_run`). One positional a command *needs* at most: optional positionals that narrow are allowed (`tm list [<kind>] [<term>]`, `every_command_takes_at_most_one_required_positional`). Both are the decided tm tree.

### The proof of the linked languages

`src/languages_test.rs` (`languages-test`, `count = { min = 0 }`) is the proof beside `src/languages.rs`: a test that builds the walks of the command line from the linked set itself and reads who declares each skipped directory, so a language that declares nothing, or a declaration that never reaches a walk, fails it. The library root declares it under `cfg(test)`.

- **One performance entry, no budgets.** `benches/performance/main.rs` (`performance-main`) is the `commands` bench (`harness = false`): it declares `measure` and one module per resource and prints one Bencher Metric Format document on stdout, which asymptote's `performance` tool judges against its lock. `measure.rs` (`performance-measure`, count 1) is the harness every product shares (rounds, CPU time per round, the document); `fixtures.rs` (`performance-fixtures`) and `use_case.rs` (`performance-use-case`) are optional (`min = 0`) and are not resource modules. A command's `bench.rs` answers `pub(crate) fn workload() -> crate::measure::Measured`. There is no divan, no `budgeted` helper and no budget constant. `benches/lint/{part}.rs` (`cli-rule-bench-part`) places the modules of the per-rule bench.

The `output` template prints an answer through `Encoded`, a trait each command's `schemas.rs` implements with `serde_json` for its own answer type, `Spacing::Compact` for `--format json` and `Spacing::Indented` where a format indents: `output.rs` names no serde, and the bytes printed are the ones serde_json wrote of the answer before.
- **Rows every command prints.** `src/output/schemas.rs` (`idiomatic.arch/schemas`, `count = { min = 0 }`) holds the wire shapes the commands share, such as the one row a violation or a record is printed as, so two commands never print one concept in two shapes; `src/output/schemas_test.rs` (`idiomatic.arch/schemas-test`) proves them, and `src/output.rs` declares the module. A row only one command prints stays in that command's schemas.rs. Edited in place.
