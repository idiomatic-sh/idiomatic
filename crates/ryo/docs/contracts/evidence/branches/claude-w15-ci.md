# Leak re-review 3: claude/w15-ci

Source `fe4f6f4d1a9a15657c5b668da8d76c6a766811c8`, base `dcea2ee7df`. This review re-checks the three variants left open at `67c935ac39`. The reviewed worktree was unchanged before and after the review (HEAD fe4f6f4d1a, clean). Plants ran in a scratch `git archive` copy and in scratch git repositories, all now deleted.

## Check 1: proof where the contract lives (PASS)
fe4f6f4d1a changes `.gitleaks.toml`, `asymptote.toml`, the ci check tests, `run.py` and `run_test.py`. No engine, core or SDK contract is asserted from `benchmarks/` or `languages/`.

## Check 2: every enforcement claim fires (PASS)

### 2c: a crashed binary is red and unowned (fixed)
When cargo's `error: test failed, to rerun pass` line follows a binary, `run.py` `test_results` records `CRASHED` for it if the binary printed no `test result:` line or no failed test. No row owns `CRASHED`.

Real `--no-fail-fast` logs (the execution gate log of 2026-11-03, lines 2952 and 3510) confirm that cargo prints this line right after each failed binary and before the next `Running`.

| Plant (over the committed record, with the standing owned `ryo_benchmark` reds) | 67c935ac39 | fe4f6f4d1a |
| --- | --- | --- |
| A `ryo_core` binary: `test t::fine ... ok`, then a stack overflow, the rerun line and SIGABRT | exit 0 | **exit 1**, `RED libraries ... owned by no row` |
| A `ryo_core` binary: `test ... FAILED`, then an abort and the rerun line | not run | `CRASHED` recorded beside the failed test, exit 1 |

**The `harness = false` exemption is sound.** A green target prints no rerun line, so nothing is recorded for it. A red target that prints no cargo-shaped output is followed by the rerun line and goes `CRASHED`, unowned. Either way it fails closed.

### 2b: an unresolvable range stops the tool (fixed)
`asymptote.toml:82` and `:101` refuse a shallow clone (exit 2). They resolve the ref with `rev-parse --verify` as a statement of its own, so `set -e` stops the tool when the ref is missing.

| Checkout | commitizen | gitleaks-history |
| --- | --- | --- |
| Full clone | 14 | 1 |
| `--depth 1` | 2 | 2 |
| `--single-branch` | 14 | 128 |
| Unknown baseline | 128 | not applicable |

### 2a: the relation id's last part (fixed)
`.gitleaks.toml:21` now accepts only `[a-z0-9]+(-[a-z0-9]+)*` as the last part. In a record folder, these keys each exit 1: `vault/prod/jwt_secret`, `stripe/live/api_key`, `aws/prod/secret_access_key`, `jwt.secret` and `stripe/api_key`. These still exit 0: `python/ruff/sequence-index-accesses` and `crates/core/src/key.rs`.

Scanning the branch's whole history with the new config (`gitleaks git --log-opts=fe4f6f4d1a`) gives 0 findings. Scanning the tree gives 0.

### Harness tests
In the archive copy, `run_test.py` runs 23 tests, all OK. `gitleaks_test.py` and `commitizen_test.py` give 18 passed.

### Advisories (not failing)
- **Truncated log:** if cargo itself is killed (exit -9, no rerun line, the log cut mid-binary), the red is still owned by the failed tests recorded before the cut. Planted, `finish()` gave 0. This needs cargo, not the test binary, to die; the OOM killer picks the larger test binary, and cargo then prints the rerun line. Cheap fix: a red command whose exit is not 101 is unowned. Candidate `red-owned-despite-truncated-log-1456be1a`.
- **Kebab-case key:** a three-part kebab-case key such as `vault/prod/jwt-secret`, holding 64 hex in a record folder, passes, because it has exactly a relation id's shape. Anchoring the first part to the language names would close it.

## Check 3: boundaries hold across crates (PASS)
No visibility, allowlist or package boundary change.

## Check 4: one implementation per concept (PASS)
The tests read the run lines from `asymptote.toml` and drive `run.py`'s own functions.

## Check 5: layer direction (PASS)
No Rust change.

## Check 6: test seams stay out of production (PASS)
No Rust change and no test seam.

## Known, not scored
`run.py --check` exits 1 on the committed record until the next validation records the `core_allocation` id.

## Enforcement candidates
Fixed:
- `secret-allowlist-unscoped-by-path-2ff533be`
- `lint-range-against-every-remote-3613f7e4`
- `red-owned-by-coarse-key-9a5b6f2b`
- `crashed-binary-hidden-behind-owned-red-ce18d982`
- `lint-range-fails-open-on-missing-ref-6b69463f`
- `secret-allowlist-key-last-part-unrestricted-4be7069e`

Open (advisory):
- `red-owned-despite-truncated-log-1456be1a`

VERDICT: PASS
