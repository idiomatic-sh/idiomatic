# Ruff benchmark

**409 PASS / 4 FAIL / 413 pinned Ruff 0.16.5 defaults.** The [complete exact-head replay](results/ci-5331-full-inventory-2026-11-03/README.md) ran every pinned rule on source `5331faa5d47c7c9a95ea741e2b763b64c9a48761`. The remaining failures are E902, I001, RUF100, and RUF200. The generated [per-rule inventory](RULES.md) records each status and its evidence. Full Ruff parity and performance acceptance remain open.

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
