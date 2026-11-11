# distribution 0.3.1

Places the Python package's `registries/pypi/LICENSE` by role, `count = { min = 0 }`, where a product listed it in its manifest. Scenario `product-s-distribution-directory-npm-pypi-cargo` holds the file. Everything else is as in 0.3.0.

# distribution

## 0.3.0

Edited in place: `npm-platform-readme` places a README in each platform package, which names the binary it carries and sends a user to the product's package; `npm-smoke-test` stages it, holds the platform tarball to it, and runs the dry-run publish with `--force`, which skips only npm's look at the registry's versions, so a version already published passes.

Edited in place: no file of the distribution lies on a ring of module dependencies (`module-cycles`). `smoke-test` names no registry's module: `REGISTRIES`, its `Install` and `Languages`, and `every_registry_is_declared` are `target-test`'s, which names each registry's `command` and `languages` through `super::`, so the registries' `super::` helpers and the root no longer name each other (the `no-registry-check` scenario moved with them). `target-preparation` holds `table` and `pinned` and names the root's `output` and `run`; its `checkout` binds the dependencies alone, and `target-test`'s `checkout` places the configuration and prepares the policy around it, so preparation names nothing of `target-test`. `bench-main` reads `target_test::REGISTRIES` and calls `target_test::checkout`.

Edited in place: no smoke test runs a target. `smoke-test` no longer has `targets_have_their_findings`, the registries'
smoke tests no longer call `installed_findings`, and `target-test` no longer has `findings` or `installed_findings`: a
smoke test proves the install, and a target's findings are the bench's, which holds every round of every target to them
through `target_test::hold`. The rule `smoke-tests-run-no-targets` holds it.

Edited in place: a target's `findings.toml` and `edit-findings.toml` hold `digest`, `exit_code` and `[rules]` alone. The
`total` is gone: it was always the sum of the counts and was checked to be, so `target-summary` computes it. A summary with
no finding leaves `[rules]` out, and `read` refuses any other key.

`target-test`'s `expected` answers whether a target states its expected findings, a `bool`: every caller asked only that, and none read the path it returned.

The four verbatim bench templates (`bench-main`, `bench-measure`, `bench-resources`, `bench-instrument`) render files
that pass `cargo-workspace` 0.11.0's lints with no finding and no `#[expect]`: every private item and field is
documented (`missing_docs_in_private_items`); `Numbers`' and `Sampled`'s fields are plain `pub` on their `pub(super)`
structs, never `pub(super)` (`field_scoped_visibility_modifiers`); no value is indexed (`indexing_slicing`): the median
and the sampler's medians read with `get`, and hyperfine's JSON with `Value::get`; no `Debug` formatting reaches
stderr (`use_debug`): lists print as `[a, b]` and durations as milliseconds; the document goes to stdout through
`writeln!`, a failed write a failure line (`print_stdout`); no result is left unused (`unused_results`): the run after
the edit applies its patch with `git apply` directly, `measure` prints the total `hold` answers as `findings=`, the
sampler's tree joins members with `extend`, and the document is collected; `CHILDREN` is a plain string
(`needless_raw_strings`); and a numeric literal states its type (`default_numeric_fallback`). The Bencher Metric
Format document, the rounds and the measures are unchanged; only lines meant for a person on stderr changed form.

The helpers return only what their callers use (`unused_results`). `smoke-test`'s `run` runs a command for its effect
and answers `io::Result<()>`; `output`, beside it, answers the command's standard output for the callers that read
it (`host`, the version and file-list checks, `git rev-parse`, `taskset`), and both report a failure through the one
private `failure`. `target-test`'s `check` answers its total to every caller; `target-preparation`'s `checkout` joins the path `relative` answers instead of calling
it for its check alone. `bench-main` applies the edit with `run` again.

A product moves to this version by changing the `version` of its `distribution` placement and applying the spec,
which rewrites `benches/bench.rs`, `benches/measure.rs`, `benches/resources.rs` and `benches/instrument.rs`.

## 0.2.0

A target's `findings.toml` and `edit-findings.toml` state `exit_code`, the exit status that run must end with by the product's own exit rule (Tastemaker 1 only when an error-level finding is expected, asymptote 1 when a new finding is, Ryo 0 for a query that ran). `target-test` and `bench-measure` hold every run to it exactly, where they accepted 0 or 1 for any run, so a command that prints its findings and ends with the wrong status fails the target; `target-summary` reads and writes the key, and refuses a summary without it.

Every product ships to Cargo; npm and PyPI are the registries a product may leave out. The `npm` and `pypi` conditions (the rule `present`, which answers yes when the registry's package file or project file is placed) make each of a registry's files required exactly when the product places that file, so a product that ships to Cargo alone (Ryo) places neither registry; a registry's targets are required by the root test (`targets` refuses a registry with none) and no longer by the spec. The `smoke-test` template declares npm's and PyPI's modules for every product but Ryo. npm's platform packages and `dist-workspace.toml`'s targets are the eight targets of the release (`x86_64` and `aarch64` of `unknown-linux-gnu`, `unknown-linux-musl`, `apple-darwin` and `pc-windows-msvc`), which maturin builds the wheels for on each target's own machine. PyPI's smoke test expects the bill of materials maturin writes, `sboms/<organization>-<product>.cyclonedx.json`.


`bench-measure` runs the bare `hyperfine` command, not `cargo bin hyperfine`: cargo-run-bin is gone, so the old call failed on a fresh clone and left `.bin/.shims` behind. `tools/install.py` installs hyperfine, which `tools/install.py` declares once. A product moves to this version by changing the `version` of its `distribution` placement and applying the spec, which rewrites `benches/measure.rs`.

The three bench templates follow the code, which had drifted from them: `bench-measure` declares `WARM_ROUNDS`, the rounds the warm run and the run after the edit are taken over (the first three of the five, the cold run taking all five), and a round's `Numbers` holds a list of each run's milliseconds, a round past `WARM_ROUNDS` holding the cold run alone; `bench-main` takes the round's `full` argument; `bench-instrument` checks the shorter rounds. The four bench templates (`bench-main`, `bench-measure`, `bench-resources`, `bench-instrument`) are now `verbatim`: a placed bench file that departs from its template by a line, a value or a declaration is reported by `#template`, where the open templates asked only that their declarations exist, and the files had gone their own way unreported. A product moves to this version by changing the `version` of its `distribution` placement and applying the spec, which rewrites `benches/bench.rs`, `benches/measure.rs` and `benches/instrument.rs`.

`smoke-test` names no product: the registries a product ships to are the entries it places (`present`): it declares `npm`'s and `pypi`'s modules, registry entries and helpers only when `npm-smoke-test` and `pypi-smoke-test` are placed, and keeps `linked` for `languages = linked`, which reads the providers a command lists in its `--version` and, where it lists none, its provider catalog. `has_version` accepts the version alone or followed by the providers in parentheses.

`find_<product>_bin()` returns the executable installed beside the package Python imported: the scripts of the scheme whose library holds the package, then the `bin` (or `Scripts`) beside a `--target` install, and only then the default scheme's and the user scheme's, so a `pip install --user` over an older system-wide wheel runs the newer binary. PyPI's smoke test installs the wheel a second time with `--target` and holds `find_<product>_bin()` to that install's executable.

## 0.1.0

The first release, edited in place by by and by. It is applied at a product's `distribution` directory, beside its command line crate, and places:

- `smoke_test.rs`: the root of the `distribution` test target, which the command line crate's manifest declares with `path = "../distribution/smoke_test.rs"` behind its `distribution` feature. It has one module per registry and the helpers they share. Its `every_registry_is_declared` test fails on a directory under `registries` it does not declare.
- `target_test.rs`: the targets' findings check the bench runs each target through. It clones each target once at its pinned revision into a cache and holds what a run prints to the target's `findings.toml`. It times nothing, and no smoke test calls it.
- `benches/`: the `distribution` bench (`bench.rs`, `harness = false`), with its measured run (`measure.rs`), its memory sampler (`resources.rs`) and its instrument's own checks (`instrument.rs`): five complete rounds of a cold run, a warm run and a run after the target's one-file edit per target, printed as Bencher Metric Format and never failing on a number; asymptote's `performance` tool judges it against the lock, so no target states a budget.
- `registries/npm/`: `dist-workspace.toml`, npm's build, the cargo-dist 0.33 build of one archive per platform (its one member is the command line crate, `cargo:../../../cli`; closed, so no downloading installer or generated CI can join it); `package.json` and its `launcher.js`, the package users install; `platforms/<triple>/package.json`, one package per platform, whose `os`, `cpu` and `libc` make npm install only the one for the user's machine; and `smoke_test.rs`, which runs the host's archive, packs the packages, holds each tarball to exactly its own files, runs `npm publish --dry-run`, installs them offline into a scratch prefix and runs `--version`.
- `registries/pypi/`: `pyproject.toml`, PyPI's build, a maturin 1.15 binary wheel built from the command line crate (`manifest-path = "../../../cli/Cargo.toml"`); `python/<organization>_<product>/`, the wheel's one flat Python module; and `smoke_test.rs`, which builds the wheel, holds it to exactly its own files, runs `uv publish --dry-run`, installs it into a fresh virtual environment and runs both entry points.
- `registries/cargo/`: `smoke_test.rs`, whose `cargo install --path` into a scratch root is Cargo's build, and which runs `--version` and `cargo publish --dry-run`. While the crate's manifest says `publish = false` the test holds Cargo to refusing the dry run for that reason; once the manifest allows publishing, the dry run must pass.
- `registries/<registry>/targets/<project>/`, at least one per registry: `pin.toml` (the `repository`, the full commit as `revision`, the `directory` the command runs in, and the `tag` when there is one), required and held by the rule `pin-revision` (there is no `budget.toml`: budgets are dropped); and `configuration.toml`, the expected findings, `edit.patch` and the expected findings after it, which a run requires and the spec places without requiring yet. The bench runs every registry's targets through the install its smoke test gives (`command`) and holds each round to the expected findings; a target with none of the four is reported and not run.

With `languages = "linked"` each registry's build links its ecosystem's languages alone, named in the build (`features` of `dist-workspace.toml` and of `pyproject.toml`, `FEATURES` of Cargo's smoke test: npm `typescript`, `javascript`, `markdown`; PyPI `python`, `toml`, `markdown`; Cargo `rust`, `toml`, `markdown`), and each smoke test's `links_its_languages` holds the build's features and the installed command's `language list` to the set the test states. With `languages = "none"` the product analyses nothing: no build names a feature and there is no language test.

The directory is closed: a registry's directory the spec does not place, a file named after a registry beside the test root, a committed binary, a second Python package beside the product's module, and a loose script or results file among the targets are findings. The product's workspace `Cargo.toml` must declare `[profile.dist]`, the profile cargo-dist and maturin build with, and the command line crate needs `serde_json`, `toml` and `tempfile` for the test target.

`distribution-npm` and `distribution-pypi`, which placed each registry's files in the command line crate, are removed: this spec places every registry.

Expected findings become summaries: a target's `findings.toml` and `edit-findings.toml` state the sha256 `digest` of the whole findings document in canonical form, the `total` and the count per rule, replacing `findings.json` and `edit-findings.json`; the `target-summary` template places `proof/summary_test.rs`, which reads them, holds each run to the digest and, on a mismatch, names the per-rule changes. The command line crate also needs `sha2` for the test target.

Performance proof adds the spec-owned preparation, resource observer and mutation-test helpers under `proof/`. Target pins declare isolated external package inputs; budgets declare fixed workers and optional cold CPU. The runner holds every one of five complete rounds to wall, CPU and sampled-tree/kernel-RSS limits and reports sampling overhead.

Findings gate and budgets bench: the `distribution` test target holds the findings alone, one run per target through the command line crate installed with the registry's languages in the workspace's `gate` profile (`cargo install --profile gate`: release's optimisation without its fat-LTO link, which a profile's findings do not depend on), and blocks only on a findings mismatch or a run error; every registry's `targets_within_budget` and `development_targets_within_budget` are removed, and so is the development run. The rounds, the budgets and the timed contract proofs move to the `distribution` bench under `benches/`, which reaches the installs and checkouts through the test root as a module; `proof/resources_test.rs` becomes `benches/resources.rs` and the timed proofs of `proof/contract_test.rs` become `benches/instrument.rs`. A target that states only its pin and its budget is reported and not run by either.

The packages carry the license: `pyproject.toml` names `license = "MIT"` and `license-files = ["LICENSE"]`, the npm smoke test stages the crate's `LICENSE` beside each `package.json` and the tarballs and the wheel are held to holding it (`package/LICENSE`, `dist-info/licenses/LICENSE`).

**The archives are named for the package**: cargo-dist names an archive for the command line package, `<organization>-<product>-<target>` (`idiomatic-sh-ryo-aarch64-apple-darwin.tar.gz`), so `npm-smoke-test` looks for that name while the binary inside stays `<product>`. `cargo-smoke-test` installs with the crate's default features alone, so `links_its_languages` fails when a default the manifest lost is a language the install no longer links.

Budgets are dropped: the rule `budget-limits` and the template `target-budget` are removed, so a target holds `pin.toml` and, for a run, its findings files and no `budget.toml`. The bench templates are `bench-main`, `bench-measure`, `bench-resources` and `bench-instrument`; the bench prints Bencher Metric Format (`cold-ms`, `warm-ms`, `edit-ms` per `<registry>/<target>`), `WORKERS` lives in `measure.rs`, and `smoke_test.rs` and `target_test.rs` no longer speak of budgets.
