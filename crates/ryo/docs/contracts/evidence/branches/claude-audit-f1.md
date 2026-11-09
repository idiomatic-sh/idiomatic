# Leak re-review: claude/audit-f1 (the Ruff benchmark), check-2 repair

Source `ad2c8867f37a0fef7ab68b86fbae8e55ec73bf95`, which supersedes the FAIL of `664f63be79`. The repair range `664f63be79..ad2c8867f3` is one commit. It touches three files: `crates/benchmark/src/application/worker_request/service_test.rs` (+137), `docs/contracts/evidence/checkpoint/commands.json` (2 lines) and `tastemaker.toml` (1 line). Nothing else changed.

The test plant ran in a detached scratch worktree with its own target dir. Both were deleted afterwards. The reviewed worktree was unchanged before and after (HEAD ad2c8867f3, status empty).

## Check 1: proof where the contract lives (PASS)
Unchanged since 664f63be79. The four moved proofs sit in their owners: query_language `fragment_contracts/test.rs:333`, SDK `fixtures/excluded_producer.rs:22` and `fixtures/configured_scope.rs:24`, and python `compiler_feature/item_test.rs:84`. None rests on a Ruff rule or corpus.

## Check 2: every enforcement claim fires (PASS)
The restored case is in `worker_request/service_test.rs`. It is registered at `:25`, dispatched at `:73`, and run by the `succeeds` test at `:166`. It reproduces the deleted `engine_test/scope_test.rs:113-143`:

| Deleted scope_test | Restored case |
| --- | --- |
| The same five passes: V/off, V/on, V/on, W/off, V/off, with a Prepared context, B002 disabled on the first file, and a retained store | `:330-336`, `:351-372` |
| A001 has 2 findings on every pass | `:381` |
| B002 has 2/1/1/0/2 findings | `:382` |
| When B002 is disabled, its remaining finding is on the second file | `:383-390`: every B002 finding is on `files[1]`; with the count of 1, this is equivalent |
| `coverage.selected` is 3 when disabled and 4 otherwise | `:391-396`: 4/3/3/4/4 |
| The source mtime is unchanged | Already covered in SDK `fixtures/configured_scope.rs` |

The case reads `filesystem.read-failure` rows of declared-missing files instead of Python functions, because the benchmark crate takes no provider. It is still a two-rule check of the hosted-table and disabled-pair path through the engine.

The case drives `prepare → engine.load → configure → execute`, which is the composition of `run_ryo/use_case.rs:36-44` without the seal check. It wires the composition and copies no production logic.

Plant: `service.rs:162` was changed to `SelectedPairs(configured.selected_pairs.0.max(rules × files))`, the unconfigured count. `succeeds` failed at `service_test.rs:393`, the coverage assertion. The plant was reverted, and the unplanted sha is green: `succeeds` and `refuses` pass. A plain replacement without the read fails earlier, with `dead_code` on `selected_pairs`.

The earlier plants still hold, because the files they cover did not change:
- `benchmark-targets@0.2.0` fired 1.
- `internal-package-allowlist` fired 1.

## Check 3: boundaries hold across crates (PASS)
`tastemaker.toml:8` now lists `idiomatic-sh-ryo-python-ruff-benchmark` in `internal-packages`, which closes the inventory advisory. The python allowlist (`:27`) and `publish = false` (`benchmarks/linters/ruff/Cargo.toml:8`) are unchanged.

## Check 4: one implementation per concept (PASS)
Unchanged since 664f63be79. The new test copies no production logic.

## Check 5: layer direction (PASS)
No production module changed in the repair.

## Check 6: test seams stay out of production (PASS)
Unchanged since 664f63be79. The new case is in a `_test.rs` module.

## Advisories
- **Stale commands: fixed.** `commands.json:24` and `:32` now name `idiomatic-sh-ryo-python-ruff-benchmark`, which declares `[[test]] ruff` and `[[bench]] ruff`.
- **Unsealed rule queries: open, and older than this branch.** 16 edited rule queries are still missing from `prepared.json`.
- **`publish = false` unenforced: open.** This is left to the TM steward.

Enforcement candidates, carried forward from the earlier review:
- `internal-package-missing-from-the-inventory-8c9a41be`: fixed
- `stale-validation-command-package-58736d1c`: fixed
- `linter-package-publishable-f479d638`: open

VERDICT: PASS
