# Ruff benchmark

**Historical milestone: 413 PASS / 0 FAIL on frozen `3ffb75b3`; this is not the current engine result.** Its [summary](results/integrated-3ff-full413-2026-11-04/summary.json) binds frozen `3ffb75b3`, sealed worker and all 11,185 cases; the other full-run receipts were pruned after `a6e880e1d7`, which keeps them. [Remote QA and Rust performance runner](https://github.com/idiomatic-sh/idiomatic/actions/runs/41850986212) targets the earlier public checkpoint and has independent status. Later cross-file scope/edit correction `cb5200df` and paired contract review remain separately gated. Broader core fixture ownership proof, memory and subsecond performance remain open. The historical 406/413 failure receipts were pruned after `a6e880e1d7`, which keeps them.

Receipts and profile variants that no code or pinned manifest reads were pruned after `a6e880e1d7`; that commit keeps them, and the shared collector's `verify-inventory` regenerates current results.

**Current engine:** full run185 recorded 411 PASS / 2 FAIL. On `f0738752d`, targeted run195 passes RUF100 (27/27 cases); B008 still fails its 120-second aggregate timeout. A new full 413-rule pass and full-Django performance acceptance remain unproven.

Run the current inventory from `crates/ryo` with the shared collector, which registers every default from this bench's `benchmark.toml`, `profiles/<id>/benchmark.toml` and `rules/<id>/`, builds and seals the worker, and writes `target/benchmark/python/ruff/verify-inventory/`:

```sh
cargo run --locked --release -p idiomatic-sh-ryo-benchmark --bin ryo-benchmark-collector -- verify-inventory languages/python/benchmarks/linters/ruff
```

That is the asymptote tool `bench-correctness/python/ruff`. The shared collector owns registration, execution receipts and PASS/FAIL decisions; missing, incomplete, invalid, or mismatched results fail. Regenerate and check [RULES.md](RULES.md) with `python3 authoring/generate_rule_inventory.py --check` after source or evidence changes.

Explicit `--prepared` and `--profile` select historical authoring profiles. Their frozen source seals can correctly reject this evolved tree. Current acceptance uses the shared collector's `verify-inventory`. `verify-reference-cases` checks pinned Ruff observations only; candidate correctness requires `verify-cases` or the full inventory.

The [historical report](results/report.md) keeps earlier correctness, split native, memory, and performance observations distinct. Their immutable evidence includes the 324-rule replay, [151-rule split proof](results/native151-supplement-2026-11-02/summary.json), memory observation, and [earlier proof refresh](results/current-proof-refresh-2026-11-01/summary.json). Those results retain their original source and worker scope.

The active pinned phase registry distinguishes the 413 shipped defaults from registered nondefault cohort definitions. Code-to-variant evidence now points to Ruff 0.16.5 `crates/ruff_linter/src/codes.rs`; the earlier active metadata mislabeled that evidence as `settings/mod.rs`. Historical receipts remain unchanged. All 413 existing phase/applicability values are preserved; supplemental registrations add no inventory credit.

Canonical `measure` and `measure-valid` cold/warm/edit benches use all413 pinned defaults, with exact rule-ID equality enforced before staging and again before timed execution. The same immutable cohort configures both contestants. `diagnostic-measure` and `diagnostic-measure-valid` take the same `--case-inventory=PATH` and may then select a comma-separated rule subset (`--case-inventory=PATH PLE2510,F401`); they certify only the selected rules' registered cases and emit sample observations but no headline benchmark metrics. Historical subset receipts remain historical and are not current full-ruleset measurements.

The canonical corpus profile explicitly declares project search roots `["", "src"]`, matching the pinned isolated Ruff scope and the reviewed I001 profile. These are host configuration, not inferred engine defaults. Profile bytes and project authority are sealed into current certification/settings identity; prior receipts retain their original profile identities. The separate Rust-source benchmark declares52 recording rules and refuses headline timing while5 remain pending; Rust speed fixtures retain their own complete configured inventories.
