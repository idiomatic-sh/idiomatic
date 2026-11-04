# Ruff benchmark

**Complete milestone: 413 PASS / 0 FAIL / 413 pinned Ruff 0.16.5 defaults.** The [source-bound full-run receipts](results/integrated-3ff-full413-2026-11-04/README.md) bind frozen `3ffb75b3`, sealed worker and all11,185 cases. [Remote QA and Rust performance runner](https://github.com/idiomatic-sh/idiomatic/actions/runs/41850986212) targets the earlier public checkpoint and has independent status. Later cross-file scope/edit correction `cb5200df` and paired contract review remain separately gated. Broader core fixture ownership proof, memory and subsecond performance remain open. Historical [406/413 failure receipts](results/integrated-a402-full413-2026-11-04/README.md) remain unchanged.

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
