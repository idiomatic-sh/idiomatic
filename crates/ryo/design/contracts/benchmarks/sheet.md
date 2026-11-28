# benchmarks
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.24. Sheet 22: every benchmark and
measuring tool we wrote, on main and on the perf tip, with what it measures, the bar it serves, the internal names it
reaches, its home in the agreed tree and how it keeps measuring the same thing through the rebuild. No line names a
`ryo_table` type, so nothing here waits on the table spike. There is no `api.txt` for this sheet (the brief names only
this file); part 5 lists what the benchmarks may name after the draft.

Revisions: MAIN `f4ae3bd802`, TIP `2a207464c`. Product code is quoted `text  @rev:path:line`. Files in the evidence
repository (`/workspaces/ryo-poc-evidence/next`) are cited as `path:line` with no `@`, because the quote checker reads
only the product repository.

## 1. Owns
The measurement of Ryo: the linter benchmarks (Python Ruff, TypeScript Oxlint, Rust Clippy) with their rule packs and
case suites, the shared harness that runs them (`benchmark`, `benchmark-collector`, `benchmark_contracts`,
`profile_tools`), the whole-run images, the per-crate micro-benches, the Asymptote ratchets that hold their numbers, and
the gate scripts in the evidence repository. It is not a crate and owns no product concept. One rule follows from the
owner's aims: **no benchmark may name an internal Ryo crate.** Today two do (`ryo_executor`, `ryo_values`); after the
draft, every benchmark names the SDK, a language's `linked()`, and process-level contracts only (part 5). That is what
lets the rebuild move every engine file without breaking a single measurement.

## 2. On main (f4ae3bd802)
What exists, by group. Lines are Rust lines with tests (`git grep -c ''` over `.rs`); tests are `#[test]` lines;
scenarios are `Scenario:` lines.

| Group | Where | Size | What it measures | Bar it serves |
|---|---|---|---|---|
| harness `benchmark` (package `idiomatic-sh-ryo-benchmark`, library `ryo_benchmark`, binary `ryo-benchmark-collector`) | `crates/ryo/crates/benchmark` | 70,746 lines (33,228 outside `*_test.rs`, `tests/`, fixtures), 300 tests, 283 scenarios, 39 public traits, 1 crate trait | verifies each rule against its cases and the reference linter, runs Ryo and the reference over each project, emits Bencher Metric Format (BMF) | the in-repo `bench` and `bench-timing` ratchets (peak, store bytes, CPU, wall, warm, edited) |
| `benchmark_contracts` (`idiomatic-sh-ryo-benchmark-contracts`) | `crates/ryo/crates/benchmark_contracts` | 4,895 lines, 32 tests, 26 scenarios, 0 traits | the typed data on the harness's process boundaries (requests, receipts, reports, workloads) | none directly |
| Ruff linter bench (`idiomatic-sh-ryo-python-ruff-benchmark`) | `crates/ryo/languages/python/benchmarks/linters/ruff` | 418 rule folders (`query.ryoql` + `cases.toml` each), 73 fragment `.ryoql` (491 rule files), 13,167 `[[cases]]`, 6,248 Rust lines, 11 tests | Ruff parity per rule and per project; Ryo against pinned Ruff on django, pandas, fastapi | parity 412/413; the Python bench rows |
| Clippy linter bench | `crates/ryo/languages/rust/benchmarks/linters/clippy` | 70 rules, 20 fragments, 1,015 cases, 2,562 lines, 4 tests | Clippy parity; ripgrep, clap | Clippy 1,015/1,015; Rust rows |
| Oxlint linter bench | `crates/ryo/languages/typescript/benchmarks/linters/oxlint` | 96 rules, 11 fragments, 2,687 cases, 2,018 lines, 4 tests | Oxlint parity; hono, vite | Oxlint 2,644/2,687; TypeScript rows |
| provider benches (divan, growth at N and 4N) | `crates/ryo/languages/{css,go,html,json,markdown,python,rust,toml,typescript}/benchmarks/provider/bench.rs`, plus python `manifest_reading`, typescript `embedded` and `manifest_reading` | 9 registered `[[bench]] provider` | reading time per language and its growth (linear check) | the parse share; no bar cell |
| crate micro-benches (divan) | `operators/benches/performance/{kernels,graphs}`; `execution_runtime/benches/performance/facts`; `core/benches/performance/{corpus,lint_store,scheduler}`; `benchmark/benches/performance/{engine,evidence,execution,linter}`; empty mains in `query_language` and `sdk` | 10 + 3 + 14 divan benches | kernels over row tables; the facts store; corpus, lint store and scheduler ports; the harness's own ports | none directly |
| Asymptote ratchets | `crates/ryo/asymptote.toml` (operations `bench`, `bench-timing`, `memory-allocation`, `test-bench`, `test-benches`); locks in `crates/ryo/.asymptote/operations/` | 62 lock files | each measure's best, by tool and benchmark name | holds the in-repo numbers |
| Tastemaker memory bench | `crates/tastemaker/cli/benches/memory` | Tastemaker product's own | `tm check` peak over a Ruff target | Tastemaker's own ratchet (out of scope here) |

Not on main: the whole-run images (`vec`, `vec_ts`, `vec_rust`, `tm`), the Tastemaker benchmark project and check
bench, the retention matrix, `profile_tools`, and every evidence-repository gate script (they were written on the perf
branch).

The internal names main's benchmarks reach (everything else they name is the SDK, `ryo_<lang>::linked*`, or the
harness itself): three constants of `values`, in the harness only.
```
ryo_values = { package = "idiomatic-sh-ryo-values", path = "../values", version = "0.2.0" }  @f4ae3bd802:crates/ryo/crates/benchmark/Cargo.toml:19
ryo_values::domain::table_words::constants::EXECUTION_TARGET;  @f4ae3bd802:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:23
ryo_values::domain::table_words::constants::RESOURCE_ROUTES_TARGET;  @f4ae3bd802:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:38
ryo_values::domain::runs::constants::SYNTAX_REJECTED  @f4ae3bd802:crates/ryo/crates/benchmark/src/domain/corpus_discrepancy/service.rs:285
```
On main those targets had emitters, so the subscriptions measured something:
```
ryo_values::domain::table_words::constants::EXECUTION_TARGET;  @f4ae3bd802:crates/ryo/crates/core/src/application/file_analysis/constants.rs:37
pub(crate) const NODE_COUNTERS_TARGET: &str = "ryo::node_counters";  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/constants.rs:10
```
The linter benches reach the harness by its use cases, for example
```
use ryo_benchmark::inbound::use_cases::run_ryo::{  @f4ae3bd802:crates/ryo/languages/python/benchmarks/linters/ruff/composition/commands.rs:20
```
and the collector was run through Cargo:
```
name = "ryo-benchmark-collector"  @f4ae3bd802:crates/ryo/crates/benchmark/Cargo.toml:72
run = ["cargo", "test", "--locked", "--package", "idiomatic-sh-ryo-core", "--all-features", "--test", "allocation", "--", "--bmf"]  @f4ae3bd802:crates/ryo/asymptote.toml:237
```
Tastemaker entries on main: `crate-benchmark`, `nu-hex-benchmark`, `idiomatic.ryo/benchmark-collector`,
`crate-benchmark-contracts`, `nu-hex-benchmark-contracts`, `benchmark-doc-page`, the `crate-layers` order (`cli` >
`benchmark` > `benchmark_contracts` > languages > `sdk`), the allowlist that limits the harness to the SDK, its
contracts and `values` (`crates/ryo/tastemaker.toml:21` at main), and `idiomatic.ryo/provider-imports` (allows
`ryo_benchmark::*` in a language's provider benches).

## 3. On the perf branch (2a207464c)

| Group | Where | Size | What it measures | Bar it serves | Runs today? |
|---|---|---|---|---|---|
| harness `benchmark` (renamed `idiomatic-sh-benchmark`, library `benchmark`, binary `benchmark-collector`, installed by `tools/install.py:102`) | `crates/ryo/crates/benchmark` | 71,904 lines (35,486 outside tests), 292 tests, 256 scenarios, 37 public traits, 1 crate trait | as on main; also `profile` (Linux observers) and runs for Asymptote and Tastemaker | the in-repo `bench`, `bench-timing`, `profile` ratchets | yes; named failure `measure_corpus` (`build/ports/stage2/gates.tsv:3`) |
| `profile_tools` (`idiomatic-sh-profile-tools`, a path package inside the harness, not a workspace member) | `crates/ryo/crates/benchmark/profile_tools` | 1,156 lines, free functions | native, sampled CPU, heap sites and user-space instructions of one command | `profile` ratchet | yes |
| `benchmark_contracts` (`idiomatic-sh-benchmark-contracts`) | `crates/ryo/crates/benchmark_contracts` | 4,986 lines, 32 tests, 26 scenarios | as on main | none | yes |
| Ruff linter bench | same path | 418 rules, 112 fragments (530 rule files), 13,186 cases, 4,831 lines, 9 tests | as on main | as on main | yes; named failures `engine_test::succeeds`, `schemas_test::succeeds` |
| Clippy, Oxlint linter benches | same paths | 70 / 96 rules, 1,015 / 2,687 cases, 2,533 / 1,989 lines, 4 / 4 tests | as on main | as on main | yes |
| whole-run lint images `vec`, `vec_ts`, `vec_rust` (package `ryo-lint-images`, own workspace, fat LTO) | `crates/ryo/crates/executor/benches/lint_image` | 483 lines | one linter request end to end through the SDK: `vec [--retain-findings] PACK REQUEST OUTPUT` | every cell of the Python, TypeScript and Rust bar rows; parity and the three scores | yes |
| Tastemaker benchmark project, image `tm` (package `tm-project-session`, own workspace) | `crates/tastemaker/benchmarks/project` (`live/`, `check_scope.py`, `replay.py`, `store_matrix.py`) | 8 files in `live/` | the product's `tm check` (keeper-live), and `--tm-batch` replays of captured SDK calls | the Tastemaker check row; identity receipts; start-up of the check | yes |
| Tastemaker check bench | `crates/tastemaker/benchmarks/check` (`bench.py`, `build.py`, `measure.py`, 294 frozen cases) | Python | release against candidate on 294 cases and the live corpus | the "5 CPU-s" target | **no**: its candidate build asks for a bench that does not exist (section 4, row G9) |
| retention matrix | `crates/ryo/benchmarks/retention/edit_matrix.py` | Python | a retaining run against ordinary evaluation on corpus copies, after edits | warm and one-edit | **no**: reads a store file nobody writes (cache sheet) |
| provider benches | `crates/ryo/languages/go/benchmarks/provider/bench.rs` registered; python `provider/performance/{fillers,manifest_reading}/bench.rs` present but **not registered** | 1 registered | go reading growth; python fill cost per element (unrun) | the fill share; no bar cell | go yes; python **no** |
| crate micro-benches | `core/benches/performance/{corpus,scheduler}`; `benchmark/benches/performance/{engine,evidence,execution,linter}`; empty mains in `query_language`, `sdk`; `cli/benches/performance` (`commands`); `distribution/benches` | 2 + 14 divan; `commands` prints BMF | corpus and scheduler ports; the harness ports; each CLI command's CPU | `performance` ratchet (CLI) | core bench does **not build** (scheduler sheet: `sequential` gone) |
| Asymptote ratchets | `crates/ryo/asymptote.toml`; 61 lock files | perf entries keyed by tool and benchmark name; 1,121 lines with `path = ` in 31 files (2,115 in 32 on main) | each measure's best | holds the in-repo numbers | three operations point at tests that no longer exist (row G10) |
| evidence gate tools | `build/ports/short-check.sh`, `build/ports/landing/landing-gate.sh`, `build/tm-int/scripts/window-nine.sh` (with `quiet-nine.py`, `startup-nine.py`, `window-warm.py`, `keeper-live.py`), `build/one-engine/scripts/{price.py, three-scores.py, price-line-run.py, price-line-report.py, price-root-split.py}`, `build/tm-int/scripts/{switch-reference.sh, full.py, tip-pack.sh}`, `build/ports/stage2/gates.tsv` | scripts | findings equal on twelve workloads; three scores; identity receipts; the whole bar table; instruction counts by bucket | **every bar cell** (the bar table of PLAN.md section 2 is measured only here) | yes, with the defects of section 4 |
| references | pinned bar binaries `/workspaces/ryo-reference-images/5e10c3cbd/{tm,vec,vec_ts,vec_rust}` (SHA256SUMS checked, `build/tm-int/scripts/window-nine.sh:18`); frozen pack `build/redo-binding/packs/c63` (digest checked, `build/tm-int/scripts/linter.py:5`); oracles `build/redo-int/runs/g59latest63-*`, `build/redo-ts/runs/project-final-*`, `build/tm-int/runs/python-cases-reference-build121.findings.jsonl`, `build/redo-honest/runs/cases/cases.json`; receipts reference `9a0c99610`; the build-139 images `/workspaces/idiomatic-target/ryo-redo-ts/landing-139/` (revision `2a207464c`, SHA256SUMS) | files | the fixed points every gate compares against | all | present (checked 28 Nov) |

**The internal names the TIP's benchmarks reach**, beyond the SDK, the languages' `linked()` and the harness: eight,
in three places.

(1) Every linter bench reads JSON through the proof of concept, for what is `serde_json::from_slice`:
```
use ryo_executor::request_input as json_input;  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/bench.rs:6
use ryo_executor::request_input as json_input;  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/tests.rs:8
use ryo_executor::request_input as json_input;  @2a207464c:crates/ryo/languages/rust/benchmarks/linters/clippy/bench.rs:6
use ryo_executor::request_input as json_input;  @2a207464c:crates/ryo/languages/typescript/benchmarks/linters/oxlint/bench.rs:6
ryo_executor = { package = "idiomatic-sh-ryo-executor", path = "../../../../../crates/executor" }  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/Cargo.toml:17
pub fn decode<T: serde::de::DeserializeOwned>(input: &[u8]) -> Result<T, serde_json::Error> {  @2a207464c:crates/ryo/crates/executor/src/request_input.rs:190
```
(2) The `tm` image replays captures through the proof of concept's own driver, bypassing the SDK, and decodes with it:
```
ryo_executor = { package = "idiomatic-sh-ryo-executor", path = "../../../../ryo/crates/executor" }  @2a207464c:crates/tastemaker/benchmarks/project/live/Cargo.toml:21
use ryo_executor::check_session::linked::Linked;  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:3
use ryo_executor::check_session::replay;  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:4
replay::batch_mode_with_workers(&readers(linked), manifest, output, None, workers, completed);  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:23
let req = ryo_executor::request_input::decode(&bytes).map_err(|e| e.to_string())?;  @2a207464c:crates/tastemaker/benchmarks/project/live/capture.rs:38
pub fn batch_mode<L:session::source::Source>(readers:&[L],manifest:&str,output:&str,root:Option<&Path>){  @2a207464c:crates/ryo/crates/executor/src/check_session/replay.rs:141
pub fn one_source<L: session::source::Source>(  @2a207464c:crates/ryo/crates/executor/src/check_session/replay.rs:302
```
It builds the executor's `Linked` from SDK methods that exist only for tests and benches:
```
pub fn parse_engine(&self) -> Arc<dyn ParseEngine> {  @2a207464c:crates/ryo/crates/sdk/src/domain/language/types.rs:142
pub fn walk_engine(&self) -> Arc<dyn WalkEngine> {  @2a207464c:crates/ryo/crates/sdk/src/domain/language/types.rs:162
pub fn project(&self) -> Arc<dyn ProjectEngine> {  @2a207464c:crates/ryo/crates/sdk/src/domain/language/types.rs:169
```
This `--tm-batch` path is what the gate's identity step (`build/tm-int/scripts/full.py:14`, `switch-reference.py`) and
the start-up of the check (`build/tm-int/scripts/startup-nine.py:54`) run. The CPU, peak, ten-worker, warm and
one-edit cells of the check run the product's `tm check --format sarif` instead (`build/tm-int/scripts/keeper-live.py:46`).

(3) The harness still takes `values` for three strings, two of which no code emits at the TIP:
```
ryo_values = { package = "idiomatic-sh-ryo-values", path = "../values", version = "0.2.0" }  @2a207464c:crates/ryo/crates/benchmark/Cargo.toml:20
ryo_values::domain::table_words::constants::EXECUTION_TARGET;  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:23
ryo_values::domain::table_words::constants::RESOURCE_ROUTES_TARGET;  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:39
ryo_values::domain::runs::constants::SYNTAX_REJECTED  @2a207464c:crates/ryo/crates/benchmark/src/domain/corpus_discrepancy/service.rs:284
pub const EXECUTION_TARGET: &str = "ryo::diagnostic::execution";  @2a207464c:crates/ryo/crates/values/src/domain/table_words/constants.rs:4
pub const SYNTAX_REJECTED: &str = "syntax-rejected";  @2a207464c:crates/ryo/crates/values/src/domain/runs/constants.rs:4
```
`git grep` for `ryo::diagnostic` and for `target: EXECUTION_TARGET`/`RESOURCE_ROUTES_TARGET` at the TIP finds only the
harness and `values`: the harness listens for the execution and resource-route events, and for its own ten
`ryo::diagnostic::*` targets, for example
```
pub(super) const RYO_DIAGNOSTIC_RULE_FILE: &str = "ryo::diagnostic::rule_file";  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/constants.rs:41
```
and nothing sends them. The node counters are the one engine event that still arrives, now from the SDK:
```
tracing::event!(name: "run", target: "ryo::node_counters", tracing::Level::INFO, runs = runs, nodes = nodes);  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:405
const NODE_COUNTERS_TARGET: &str = "ryo::node_counters";  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/writer.rs:24
if row.target != "ryo::node_counters" {  @2a207464c:crates/ryo/crates/benchmark/src/application/profile_evidence.rs:325
```

**What everything else reaches, the process-level contracts** (these are what must not change):
```
ryo = { package = "idiomatic-sh-ryo-sdk", path = "../../../sdk", features = ["bench-hooks"] }  @2a207464c:crates/ryo/crates/executor/benches/lint_image/Cargo.toml:28
pub fn run(  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:39
sdk::Cache::At(request.root.join(".ryo_cache"))  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:122
let engine = sdk::Linter::new(  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:144
eprintln!("HARNESS_STARTUP process_cpu={:.6} request={parsed:.6} provider={provider:.6}", clock());  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:152
sdk::print_measurement();  @2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:157
pub fn print_measurement() {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session.rs:42
ryo_sdk::linter::print_measurement();  @2a207464c:crates/tastemaker/benchmarks/project/live/tm_main.rs:72
.find_map(|line| line.strip_prefix("CHECK_SESSION live "))  @2a207464c:crates/ryo/crates/benchmark/src/application/profile_evidence.rs:253
if raw.startswith(b"CHECK_SESSION "):  @2a207464c:crates/tastemaker/benchmarks/check/measure.py:66
let built = linter::Linter::new(  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:63
let report = engine.lint(vec![LintRequest {  @2a207464c:crates/ryo/crates/benchmark/src/outbound/engine/adapters/local/adapter.rs:206
```
`print_measurement` writes the stderr lines `UNANSWERED`, `ADMISSION_EPOCH`, `TM_ROUND_COUNTS` and `CHECK_SESSION live
calls=... prep=... read_parse=... first=... second=... project=... assembly=...`
(`crates/ryo/crates/sdk/src/resources/linter/lint/session/measure.rs:47-75` at TIP); the evidence scripts and the
Tastemaker check parse them by name.

The harness is shared by three products through the installed collector:
```
name = "benchmark-collector"  @2a207464c:crates/ryo/crates/benchmark/Cargo.toml:73
run = ["benchmark-collector", "profile", "distribution/benches/profile"]  @2a207464c:crates/asymptote/asymptote.toml:195
run = ["benchmark-collector", "profile", "benchmarks"]  @2a207464c:crates/tastemaker/asymptote.toml:210
run = ["taskset", "-c", "0-3", "env", "RAYON_NUM_THREADS=4", "benchmark-collector", "bench", "languages/{lang}/benchmarks/linters/{linter}"]  @2a207464c:crates/ryo/asymptote.toml:284
run = ["benchmark-collector", "profile", "languages/{lang}/benchmarks/linters/{linter}"]  @2a207464c:crates/ryo/asymptote.toml:228
```
The images are built from source at a revision by one script, which knows the image package's path:
```
vec=$src/crates/ryo/crates/executor/benches/lint_image  @2a207464c:crates/tastemaker/benchmarks/project/live/images.sh:25
const IMAGES: [&str; 4] = ["tm", "vec", "vec_ts", "vec_rust"];  @2a207464c:crates/ryo/crates/executor/tests/image_sources.rs:5
```
Tastemaker entries at TIP: as on main with the new names (`crates/ryo/tastemaker.toml:110-112, 190-191`), the
allowlist still granting `idiomatic-sh-ryo-values` (`crates/ryo/tastemaker.toml:19`), `crate-layers` with `executor`
between `sdk` and `core` (`:35`), and `provider-imports` allowing `benchmark::*` (`:185`).

## 4. What changed between them
Added on the TIP: the four images and `images.sh`; the Tastemaker benchmark project and check bench; the retention
matrix; `profile_tools` and the collector's `profile` command (5e2517b3fb "Profile registered benchmarks with the shared
collector"); the collector renamed and installed (dace9d9ada "Rename the shared benchmark collector and contracts",
057eace9b3 "Register Asymptote and Tastemaker with the installed benchmark collector"); the CLI `commands` bench and the
`distribution` bench; the python `fillers` bench (6be47f3720); 19 more Ruff cases (13,167 to 13,186) and 39 more Ruff
fragments.

Changed: the linter benches took `ryo_executor` as a dev-dependency for JSON (3b089feb69 "linter benchmark crates read
the executor's request_input; the executor is their dev-dependency"); Clippy and Oxlint dropped `serde_json`
(`git diff` of their `Cargo.toml`); the per-linter `composition/profile.rs` went (the collector profiles instead).

Removed, and what was lost that should not have been:
- **Two designed contracts of the harness**, in 5e2517b3fb:
  ```
  pub trait ProfileUseCase {  @f4ae3bd802:crates/ryo/crates/benchmark/src/inbound/use_cases/profile/interface.rs:5
  pub trait ProfileToolsPort {  @f4ae3bd802:crates/ryo/crates/benchmark/src/outbound/profile_tools/interface.rs:9
  ```
  replaced by free functions in `profile_tools`:
  ```
  pub enum Observer {  @2a207464c:crates/ryo/crates/benchmark/profile_tools/src/lib.rs:39
  ```
  Decision 4.
- **Eight of the nine provider benches** (python 7dac50ba0e, rust 55edecb98a, html 00d0d00c0a, typescript ed1e28238a
  "the old reader is deleted"; css, json, markdown, toml not traced), and typescript `embedded`/`manifest_reading`.
  Python's two remaining files are no longer registered. Main had the first line below; the TIP's manifest has no
  `[[bench]]` and the second, so `fillers` never compiles:
  ```
  path = "benchmarks/provider/bench.rs"  @f4ae3bd802:crates/ryo/languages/python/Cargo.toml:54
  autobenches = false  @2a207464c:crates/ryo/languages/python/Cargo.toml:12
  ``` **No Asymptote lock at either revision holds `ns-per-element` or `growth-percent`**
  (`grep` over all 62 and 61 lock files): the table, relations, language and store sheets say these measures are
  "held by Asymptote"; that is not so.
- The micro-benches over the deleted crates: operators `kernels` and `graphs`, execution_runtime `facts`, core
  `lint_store`. Nothing at the TIP benches a kernel, the store or the kept cache alone (table, operators, store, cache
  sheets).
- Ruff bench tests 11 to 9: `project_context_test.rs` (244 lines), `engine_test/{i001_observation_test.rs (345),
  rejected_syntax_consumer_test.rs (258), consumer_selection_test.rs (173), constants.rs (62)}` deleted.
- The harness's diagnostic subscriptions lost their emitters (section 3, (3)).

**The gate's defects, as rows** (PLAN.md section 9 plus what this pass found; G1 to G7 are the plan's, each
re-checked here):

| # | Tool | Defect | Evidence | Fix before the gate is trusted |
|---|---|---|---|---|
| G1 | `build/one-engine/scripts/build.sh` | builds whatever tree its stage was first linked from, not `INTEGRATOR_TREE`; also runs `link-release.py` and compiles `crates/ryo/crates/engine/tests/image_sources.rs`, neither of which exists at the TIP (the test is in `executor/tests/`; `engine/` holds only `benches/lint_image/Cargo.lock`) | `build/one-engine/scripts/build.sh:4-11, 18`; `discovery-inputs/code-review-gate-scripts.md:14-18` | retire it: `images.sh` is the one builder |
| G2 | `build/tm-int/scripts/full.py` | the `ryo` workspace's answers are checked by row count (46,831) unless the image's sha is one pinned value | `build/tm-int/scripts/full.py:36-39` | compare against the current reference for every sha, read from `check-session/current-reference.txt` |
| G3 | `host3/scripts/warm-check` | `findings_same=$?` reads `cat`'s status, so it always prints 0 | `host3/scripts/warm-check:21` | retire (not in any gate since 20 Nov); `window-warm.py` replaced it |
| G4 | `build/ports/landing/landing-gate.sh` | runs no package tests; and every step ends `; echo "step N exit $?"`, so the script always exits 0 | `build/ports/landing/landing-gate.sh:12-19` | add the test step; exit non-zero when any step fails |
| G5 | `build/ports/short-check.sh` | the identity step compares against the newest pre-switch image, which is `build133` (the folder holds `build129`, `build133`); the two `git archive | tar` lines run outside `step`, their status unchecked | `build/ports/short-check.sh:51, 53, 56` | pin the pre-switch image by path and digest; check the archives |
| G6 | five scripts | hard-code today's paths: the three pack folders `crates/ryo/languages/<lang>/benchmarks/linters`, the repository `/workspaces/ryo-redo-empty`, the images package `crates/ryo/crates/executor/benches/lint_image`, the evidence wait script | `build/ports/short-check.sh:22, 53-55`; `build/tm-int/scripts/tip-pack.sh:5-11`; `build/one-engine/scripts/price.py:16, 70`; `build/one-engine/scripts/three-scores.py:16, 41`; `images.sh:18, 25` (TIP, quoted above) | one place for the pack paths: `tip-pack.sh` prints them and the other four read its exports only; `images.sh` reads the images package path from one variable |
| G7 | instruction counts by bucket | `price-root-split.py`'s rows match module names (`ryo_query_language|plan_lowering`, `check_session::`, `pair_summary`, `finish::`), `price-line-report.py`'s `own()` buckets by file basenames (`check_session.rs`, `pair_summary.rs`, `reader_run.rs`), and `price-library-callers.py` names paths gone even at the TIP (`python/benchmarks/vec/bench.rs`, `/crates/engine/benches/vec/`) | `build/one-engine/scripts/price-root-split.py:10-22`; `build/one-engine/scripts/price-line-report.py:50-80`; `build/one-engine/scripts/price-library-callers.py:83, 90` | after the rebuild a bucket is a crate: match `ryo_<crate>::` (one row per crate of the tree), and keep one translation table from build 139's buckets to crates so the two sides can be compared |
| G8 | the plan | PLAN.md section 7 step 0 and section 9 say instruction counts come from `price.py`; `price.py` measures findings equality, CPU and peak (`wait4`), no instructions; the instructions come from Callgrind in `price-line-run.py` | `build/one-engine/scripts/price.py:91-99`; `build/one-engine/scripts/price-line-run.py:71-73` | correct the plan; the spike's "+0.2% instructions" is measured with `price-line-run.py` |
| G9 | Tastemaker check bench | its candidate build asks for `--bench vec-readers` of the command line, which has only `commands` and `distribution` at the TIP; the project README names `link-release.py` and `vec-readers`, absent too | quoted below the table | build the candidate from `images.sh tm` (the same binary the gate measures) |
| G10 | Asymptote | three operations name tests deleted on the branch: `memory-allocation` (`core/tests/allocation.rs`), `test-boundary-contract`, `test-execution-use-case` (`execution/tests/boundary_contract`, `inbound::use_cases::execution`); the last two are `--no-tests=fail` | quoted below the table | point each at the restored test of its new crate (store sheet's allocation test; execute's contract tests), never delete the operation |
| G11 | python `fillers` | unregistered; no lock holds its measures | section 4 above | register it as `[[bench]]` of the python crate and seed its lock |
| G12 | the reference images | the build-139 images live in a cargo target folder (`/workspaces/idiomatic-target/ryo-redo-ts/landing-139/`); PLAN.md section 3 calls the folder `landing-13x/`, which does not exist | `ls` on 28 Nov | copy them to `/workspaces/ryo-reference-images/2a207464c/` with `SHA256SUMS`, as `5e10c3cbd` is kept, and check the digest in the gate |
| G13 | `window-nine.sh` | its bar is the pinned `5e10c3cbd` binaries, not build 139; nothing computes "new tree against build 139, a cell more than 0.5% worse fails" | `build/tm-int/scripts/window-nine.sh:9, 18` | run both image sets in one window (`b139=landing-139@2a207464c,new=DIR@REV`, the script's list form, `:5`) and add the one comparison that fails a cell |
| G14 | dagster | the goal bench is not in the in-repo suite: `benchmark.toml` lists django, pandas, fastapi; dagster exists only in the evidence scripts | `crates/ryo/languages/python/benchmarks/benchmark.toml` at both revisions | none for the rebuild (the gate is the evidence scripts); recorded so nobody reads the in-repo ratchet as covering the goal |
| G15 | Ruff score | `three-scores.py` scores Ruff from the frozen `redo-honest/runs/cases/cases.json` (13,167), not from the pack's own 13,186 cases; Oxlint and Clippy are scored from the pack's `cases.toml` | `build/one-engine/scripts/three-scores.py:28-37, 40-54` | keep it (it is a fixed yardstick); state it beside the score |
| G16 | `gates.tsv` | the build-139 row was not taken quiet ("NO except tm ten workers and start-up") | `build/ports/stage2/gates.tsv:3` | take build 139's cells again in the same quiet window as the new tree (G13) |

G9 and G10, quoted:
```
"--features", "idiomatic-sh-ryo-python/fixtures", "--bench", "vec-readers"],  @2a207464c:crates/tastemaker/benchmarks/check/build.py:100
name = "commands"  @2a207464c:crates/ryo/cli/Cargo.toml:75
run = ["cargo", "test", "--locked", "--package", "idiomatic-sh-ryo-core", "--all-features", "--test", "allocation", "--", "--bmf"]  @2a207464c:crates/ryo/asymptote.toml:260
run = ["cargo", "nextest", "run", "--workspace", "--locked", "--all-features", "--no-tests=fail", "--run-ignored", "all", "-E", "package(=idiomatic-sh-ryo-execution) & kind(test) & binary(=boundary_contract)"]  @2a207464c:crates/ryo/asymptote.toml:101
run = ["cargo", "nextest", "run", "--workspace", "--locked", "--all-features", "--no-tests=fail", "--run-ignored", "all", "-E", "package(=idiomatic-sh-ryo-execution) & kind(lib) & test(/^inbound::use_cases::execution::/)"]  @2a207464c:crates/ryo/asymptote.toml:109
```

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
The benchmarks offer nothing to Ryo; what is drafted is **what they may name in Ryo**. After the draft:

```
(a) the SDK, public
pub fn ryo::linter::Linter::new(languages, pool, cache) -> Linter                     [main/perf: owner-fixed surface, 24 Nov]
pub fn ryo::linter::Linter::lint(&self, Vec<LintRequest>) -> Vec<Result<LintReport, LintError>>
the request/report types, Cache::{None, At}, Languages, WorkerPool::from(Arc<rayon::ThreadPool>)
each language's linked() / linked_with_settings(..)                                   [unchanged]
(b) the SDK, feature bench-hooks
pub fn ryo::linter::print_measurement()                                               [perf, unchanged]
(c) process contracts, unchanged in name and meaning
vec|vec_ts|vec_rust [--retain-findings] PACK REQUEST OUTPUT; findings JSONL out
tm check --format sarif; tm --tm-batch MANIFEST OUT [--workers N]; --tm-batch-retained ROOT MANIFEST OUT; --mixed PACK REQUEST OUT
stderr: HARNESS_STARTUP, CHECK_SESSION live <keys>, UNANSWERED, ADMISSION_EPOCH, TM_ROUND_COUNTS
tracing: target "ryo::node_counters", event "run", fields runs, nodes
(d) stops being named by any benchmark
ryo_executor::request_input (decode)                          -> serde_json::from_slice, as on main
ryo_executor::check_session::{replay, linked::Linked}         -> decision 1
ryo_values::domain::table_words::constants::{EXECUTION_TARGET, RESOURCE_ROUTES_TARGET} -> the harness's own literals (decision 3)
ryo_values::domain::runs::constants::SYNTAX_REJECTED          -> the harness's own literal (decision 3)
Language::{parse_engine, walk_engine, project} under bench-hooks -> no benchmark caller left (the language sheet decides their fate under fixtures)
```

Counts of internal (non-SDK) Ryo names a benchmark reaches: **main 3** (the `values` constants), **TIP 8** (the three
constants, `request_input::decode`, `replay::{batch_mode, batch_mode_with_workers, one_source}`, `Linked`), **draft 0**.
The harness's own surface (used by the linter benches only) is not part of Ryo's tree: 39 public traits on main, 37 at
the TIP, 39 drafted if decision 4 restores the two.

Homes in the agreed tree (PLAN.md section 4, Appendix A section 1: tools go to top-level `tools/`, unpublished; each
crate folder carries its package name):

| What | Home after the draft | Why |
|---|---|---|
| `benchmark` + `benchmark-collector` | `tools/benchmark` (package `idiomatic-sh-benchmark`, unchanged) | shared by Ryo, Tastemaker and Asymptote (section 3) |
| `profile_tools` | `tools/profile_tools` | a library of the harness; no engine dependency (its own doc) |
| `benchmark_contracts` | `tools/benchmark_contracts` | the harness's data; depends on the SDK only |
| Ruff, Clippy, Oxlint benches, packs, cases | inside their language's crate folder, `.../languages/idiomatic-sh-ryo-{python,rust,typescript}/benchmarks/linters/{ruff,clippy,oxlint}` | benches stay with their product and language; one path change, made once (G6) |
| lint images `vec`, `vec_ts`, `vec_rust` | `crates/ryo/benchmarks/lint_images` (own workspace, as today), beside `retention` | `executor` goes away; they measure Ryo through the SDK only (decision 2) |
| `image_sources` test | inside the lint images package | it tests the images, not an engine crate |
| `tm` image (`tm-project-session`), Tastemaker check bench | `crates/tastemaker/benchmarks/{project,check}`, unchanged | Tastemaker's own; patch table re-pointed at the new crate folders |
| retention matrix | `crates/ryo/benchmarks/retention`, unchanged | names no Rust type |
| provider benches | each language's `benchmarks/provider` | languages sheet |
| crate micro-benches | in their crate: corpus and scheduler benches to those crates; `kernels`/`graphs` to operators; `facts` to store; `lint_store` to cache; `commands` and `distribution` with the command line | each sheet says so (section 8) |
| Asymptote locks | `crates/ryo/.asymptote/operations/` | perf entries are keyed by tool and benchmark name (for example `{ tool = "bench/python/ruff", benchmark = "python/django", measure = "peak-rss-bytes" }`), so they survive; only the path-keyed suppression entries are re-keyed |
| evidence gate tools and references | the evidence repository, unchanged | outside the product |

## 6. Proof it brings
Carried as they are: the harness's 292 tests and 256 scenarios, `benchmark_contracts`' 32 and 26, the linter benches'
9 + 4 + 4 tests and their 13,186 + 1,015 + 2,687 cases, the 294 frozen Tastemaker cases, `image_sources`.
Restated: none (no benchmark test names an engine type once part 5 (d) is done).
The three named failures (the harness's `measure_corpus` and the two Ruff `succeeds` below) are carried as known,
compared by name (PLAN.md section 9):
```
fn succeeds() {  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/engine_test.rs:223
fn succeeds() {  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/schemas_test.rs:143
```
Missing: (1) any test of the process contracts in part 5 (c), which the evidence scripts, the harness and the Tastemaker check parse by name: one smoke
test in the lint images package that runs `vec` on a two-file corpus and checks the `HARNESS_STARTUP` and `CHECK_SESSION
live` keys would hold them; (2) a test that the gate fails: each of G2, G4, G5, G13 gets one planted failure (a changed
answer, a failing step, a wrong pre-switch image, a cell 1% worse) that must turn the gate red; (3) the lost Ruff bench
tests (section 4): restore from main where the behaviour still exists (the observation and rejected-syntax consumer
cases), else record why not.

## 7. Perf gains it carries
Code of its own: one gain, partly. Row 42's second half ("the step clock stops calling the system", Django 32.68 to
24.82 with the union and antijoin kernels) is the thread-CPU clock the session reads
(`crates/ryo/crates/executor/src/bench_support.rs` at TIP, named by the SDK's `session.rs`); it goes with execute's
clock (execute sheet), not with a benchmark.

What it owes aim 1 is the measurement of every live row. Each live row of `perf-gains.tsv`, by the tool that holds it
(grouped from the row's measured effect; not re-derived from sources):

| Bar cell | Tool, after the draft unchanged | Live rows it holds |
|---|---|---|
| Python CPU and peak, one worker | `quiet-nine.py` with `vec` (django, pandas, fastapi, dagster) | 3, 6, 7, 24, 29, 31, 32, 33, 35, 36, 37, 38, 39, 41, 42, 43, 44, 45, 54 |
| TypeScript and Rust CPU and peak | `quiet-nine.py` with `vec_ts`, `vec_rust` | 5 (hono), 11, 28 |
| the Tastemaker check CPU and peak | `quiet-nine.py`/`landing-pairs.sh` with `tm check` | 1, 4, 5 (Go check), 9, 16, 17, 18, 19, 20, 21, 22, 23, 51, 52 |
| ten workers | `quiet-nine.py` ten-core part | 54; parked 13 |
| start-up | `startup-nine.py` (zero-file request; `tm --tm-batch` empty manifest) | 1, 6, 25, 40 |
| warm, one edit, store size | `window-warm.py` (`vec --retain-findings`, `tm check` warm and edited) | 2, 14, 52; parked 53 |
| counters only (no CPU cell) | `CHECK_SESSION` via `print_measurement` | 11, 12, 19, 20, 22, 49, 50 |
| instructions | Callgrind, `price-line-run.py` | 8, 10, 30 |
Every row's measurement goes through an image that names only the SDK after part 5 (d). Row 51's stat-family syscall
count has no script I could find (its CPU share is the check's row).

Gains from the history that would have landed inside the benchmarks alone under this draft: 2 and 52 (composition of
the bench and the product check: the edge, not an engine crate), 17 (the check's rounds: Tastemaker's adapter).
Gains that crossed into a benchmark and would not under the draft: 25 ("JSON plan reader removed; provider columns
borrowed" touched `request_input` that the linter benches then imported, 3b089feb69) and every executor move that broke
`capture.rs`'s path (f2869d1f74 "move the vector executor", renames only): once no benchmark names an engine crate, an
engine move cannot touch a benchmark.

## 8. Benchmarks that cover it
This is the inventory sheet; every other sheet's part 8 points here. Where a sheet's part 8 and this sheet differ:
- table, relations, language, store sheets: "python `fillers` ... held by Asymptote": the bench is unregistered and no
  lock holds its measures (G11).
- table sheet 8b and others: the images are "`executor/benches/lint_image`": they move (decision 2).
- programs sheet: the Tastemaker benchmark project's replay reaches the TIP's preparation by name (`capture.rs`); that is
  decision 1 here.
- scheduler sheet decision 5: the harness's own `SchedulerPort` (a thread per task) stays in `tools/benchmark`,
  unchanged:
  ```
  pub trait SchedulerPort: Send + Sync {  @2a207464c:crates/ryo/crates/benchmark/src/outbound/scheduler/interface.rs:4
  Arc::new(benchmark::outbound::scheduler::factory::threads()),  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/composition/measure.rs:168
  ```

## 9. Checks
IMPORTS: sdk, python, typescript, rust (the linter benches and the images; the `tm` image also the other six languages and Tastemaker); `tools/` sits outside the tree's directions
CROSSED: per request
- No layout word: no benchmark names a table, a column or a row type after part 5 (d).
- How often: a benchmark crosses into Ryo once per `lint` call (per request); the images once per process.
- Held by: Cargo (no manifest of a benchmark lists an engine crate: the executor lines in four `Cargo.toml` go); the
  Tastemaker allowlist for the harness loses `idiomatic-sh-ryo-values` (`crates/ryo/tastemaker.toml:19` at TIP);
  `provider-imports` stays; a new rule is not needed.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **The `tm` image's replay.** Example: `capture.rs:3-4, 23` calls `ryo_executor::check_session::replay` directly, and
   the identity step of the gate (receipts in both orders, the 294 cases, `full.py:14`) and the start-up of the check
   (`startup-nine.py:54`) run that path, not the SDK. Why: the executor goes away; a benchmark that drives an engine
   driver directly is a second path and breaks on every engine move. What we get: one path measured. My pick: the
   bench package turns each captured line into `LintRequest`s and calls `Linter::lint` in the captured rounds; the
   `--tm-batch` flags keep their names and output, so no script changes. The receipts must match as before; start-up
   with an empty manifest admits nothing, so the build-139 figure stays comparable.
2. **Where the lint images go.** Example: `images.sh:25` builds `crates/ryo/crates/executor/benches/lint_image`. Why: the
   folder's crate is deleted, and the images name only the SDK and three languages. My pick:
   `crates/ryo/benchmarks/lint_images`, its own workspace as today, beside `retention`; `images.sh` reads the path
   from one variable (G6). Alternative: `tools/`; rejected because the images measure Ryo only.
3. **The harness's three `values` strings.** Example: `writer.rs:23, 39`, `corpus_discrepancy/service.rs:284` at TIP.
   Why: `values` is split by the rebuild, and two of the three name events nobody emits. My pick: the harness holds
   the three strings itself, as it already holds its ten `RYO_DIAGNOSTIC_*` targets; whether a new crate emits the
   execution and resource-route events again is for the execute and lint sheets. Nothing is deleted.
4. **`ProfileUseCase` and `ProfileToolsPort`, removed in 5e2517b3fb.** Why: a designed contract is never dropped
   without the owner. My pick: restore `ProfileToolsPort` in the harness with `profile_tools` as its one Linux adapter
   (the library stays free of benchmark types); `ProfileUseCase` returns as the collector's `profile` command's use
   case. Otherwise the ledger records both as dropped in your words.
5. **Repair the gate before the build, as one batch.** Example: `landing-gate.sh` always exits 0 (G4); the build-139
   images sit in a target folder (G12); the instruction buckets name file basenames (G7). Why: aim 1 is judged by
   this gate, and today it can pass a broken tree. My pick: retire G1 and G3; fix G2, G4, G5, G6, G7, G10, G11, G13
   with one planted failure each (part 6); copy the build-139 images to `/workspaces/ryo-reference-images/2a207464c/`;
   then re-take build 139's cells in a quiet window (G16).

## 11. Not checked
- Whether the in-repo `bench` and `bench-timing` operations pass at the TIP today, and whether their lock entries were
  seeded on this machine or a CI runner: not run (no Cargo).
- The 1,073 path-keyed entries in 30 files of PLAN.md section 8.2: not reproduced; counting lines with `path = ` gives
  2,115 in 32 files on main and 1,121 in 31 at TIP, which also counts removals. Which entries are live after the
  chain is applied was not computed.
- Which of the eight lost provider benches should come back: left to the languages sheet. The commits that removed
  css, json, markdown and toml were not traced.
- The Tastemaker memory bench on main and Tastemaker's own `benches/performance` at TIP: Tastemaker's, not read.
- The gain-to-tool table in part 7 is grouped from each row's measured-effect text, not from the row's source; row
  30's "fitted fixed cost" and row 51's syscall counts have no standing tool I could find.
- `price-line-run.py` and `price-line-report.py` were read only around the Callgrind call and `own()`; how
  `price-root-split.py` assigns library instructions to callers was not checked.
- `check_scope.py`, `replay.py`, `store_matrix.py` and the rest of `crates/tastemaker/benchmarks/project`: only
  `live/` was read.
- The harness's lines "outside tests" (35,486 at TIP) use my filter (`_test.rs`, `/tests/`, `_test/`, `fixtures`);
  PLAN.md's 35,008 uses another; neither is authoritative.
- Why the harness names `ryo::adapter::LinterRyoAdapter` and `ryo::identity::schemas` (one hit each): likely doc text;
  not opened.
