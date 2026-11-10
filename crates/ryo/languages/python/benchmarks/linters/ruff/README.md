# Ruff benchmark

**Historical milestone: 413 PASS / 0 FAIL on frozen `3ffb75b3`; this is not the current engine result.** The [source-bound full-run receipts](results/integrated-3ff-full413-2026-11-04/README.md) bind frozen `3ffb75b3`, sealed worker and all11,185 cases. [Remote QA and Rust performance runner](https://github.com/idiomatic-sh/idiomatic/actions/runs/41850986212) targets the earlier public checkpoint and has independent status. Later cross-file scope/edit correction `cb5200df` and paired contract review remain separately gated. Broader core fixture ownership proof, memory and subsecond performance remain open. Historical [406/413 failure receipts](results/integrated-a402-full413-2026-11-04/README.md) remain unchanged.

**Current engine:** full run185 recorded 411 PASS / 2 FAIL. On `f0738752d`, targeted run195 passes RUF100 (27/27 cases); B008 still fails its 120-second aggregate timeout. A new full 413-rule pass and full-Django performance acceptance remain unproven. See the [contract evidence](../../../../../docs/contracts/README.md).

Run the current inventory from this directory with the shared runner:

```sh
python3 verify_profiles.py /absolute/path/to/pinned-ruff --full-inventory \
  --worker /absolute/path/to/sealed-worker \
  --inventory-runner /absolute/path/to/ryo-benchmark-collector \
  --output-dir /fresh/output
```

Without `--worker`, the adapter builds and seals one. Positional rule IDs select cases while the result still contains all 413 rows; unselected rules remain FAIL. The shared collector owns execution receipts and PASS/FAIL decisions. Missing, incomplete, invalid, or mismatched results fail. Regenerate and check [RULES.md](RULES.md) with `python3 authoring/generate_rule_inventory.py --check` after source or evidence changes.

Explicit `--prepared` and `--profile` select historical authoring profiles. Their frozen source seals can correctly reject this evolved tree. Current acceptance uses the shared full-inventory route. `verify-reference-cases` checks pinned Ruff observations only; candidate correctness requires `verify-cases` or the full inventory.

The [historical report](results/report.md) keeps earlier correctness, split native, memory, and performance observations distinct. Their immutable evidence includes the [324-rule replay](results/correctness-324-final-2026-11-01.json), [151-rule split proof](results/native151-supplement-2026-11-02/summary.json), [memory observation](results/memory-ownership-measurements-2026-11-01.json), and [earlier proof refresh](results/current-proof-refresh-2026-11-01/summary.json). Those results retain their original source and worker scope. The [engine ownership checklist](../../../../../docs/contracts/README.md) tracks current responsibilities and open gates.

The active pinned phase registry distinguishes the 413 shipped defaults from registered nondefault cohort definitions. Code-to-variant evidence now points to Ruff 0.16.5 `crates/ruff_linter/src/codes.rs`; the earlier active metadata mislabeled that evidence as `settings/mod.rs`. Historical receipts remain unchanged. All 413 existing phase/applicability values are preserved; supplemental registrations add no inventory credit.

Canonical `measure` and `measure-valid` cold/warm/edit benches use all413 pinned defaults, with exact rule-ID equality enforced before staging and again before timed execution. The same immutable cohort configures both contestants. `diagnostic-measure` and `diagnostic-measure-valid` may select a diagnostic subset; they emit sample observations but no headline benchmark metrics. Historical subset receipts remain historical and are not current full-ruleset measurements.

Whole-Django parity (the `parity` command, asymptote tool `performance-python-ruff-parity`) needs one setup step from `crates/ryo`: `ln -sf <pinned-django-snapshot>/cold-request.json target/benchmark/python/django/cold-request.json`; the corpus stays outside the repository because of its size, and the corpus digest the accepted differences name ties every run to it.

The canonical corpus profile explicitly declares project search roots `["", "src"]`, matching the pinned isolated Ruff scope and the reviewed I001 profile. These are host configuration, not inferred engine defaults. Profile bytes and project authority are sealed into current certification/settings identity; prior receipts retain their original profile identities. The separate Rust-source benchmark declares52 recording rules and refuses headline timing while5 remain pending; Rust speed fixtures retain their own complete configured inventories.
