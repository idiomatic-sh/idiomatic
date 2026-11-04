# Ruff benchmark

**Latest complete integrated run: 406 PASS / 7 FAIL / 413 pinned Ruff 0.16.5 defaults.** The [source-bound checkpoint](results/integrated-a402-full413-2026-11-04/README.md) ran every rule on `a402537b`. F541 admission, PLW0133/UP035 stack depth, RUF028/UP034 dependency readiness, RUF100 producer readiness, and UP031 archived identity failed. F541 later passes21 targeted cases; current source followups are not a new complete run. I001's precursor passed, but its N3 correction still needs fresh proof. The generated [per-rule inventory](RULES.md) distinguishes stale source evidence. Full parity and contract closure remain open.

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
