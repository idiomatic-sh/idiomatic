# Ruff benchmark integration status

**Status: in progress. Full current-corpus parity and a controlled current performance result are not established.** The active canonical cohort is prepared and sealed, but the candidate has not yet completed it.

## Active cohort

The active descriptor is `current-canonical-211-refresh`: 211 selected Ruff defaults, 3,827 canonical fixture cases, and 4,815 pinned Ruff diagnostics under PY310. It uses canonical `rules/<CODE>/candidate.ryoql` or `query.ryoql` and `cases.toml` inputs. The oracle came from one pinned Ruff 0.16.5 invocation over those current source bytes; reference return code 1 is expected because Ruff reported findings. The descriptor passed manifest verification and combined-profile staging. This is oracle/preflight evidence, not candidate parity.

- Oracle: [current-canonical-211-refresh/oracle.json](../profiles/current-canonical-211-refresh/oracle.json), SHA-256 `7b47e7ac0651af75ef26b1d466f92cd0a8e47a45937a7e254ce58127824c8974`.
- Expectations: [current-canonical-211-refresh/expectations.json](../profiles/current-canonical-211-refresh/expectations.json), SHA-256 `6f0cbb09140fcf273f65a1c7d472538eab96bf6d042a78731cddedfd755ec1c5`.
- Prepared manifest SHA-256 at this checkpoint: `6931f1e427ef35af1d5cb20b42c06aedf09164735ce4ff6803fd1b377593f311`.

Historical `shape-110`, `expanded-defaults`, `current-canonical` (205 rules), and `current-canonical-211` (older 211-rule/3,766-case inventory) remain archived separately. Their original oracle and result artifacts are preserved; the 205- and 211-rule descriptors resolve to sealed snapshots where historical inputs differ from current canonical files. They are not the active acceptance cohort.

## Scoped candidate evidence

The latest frozen-worker batch recorded 43 isolated profiles across 619 distinct fixture cases, with 1,238 reference/candidate case records passing. This evidence combines worker revisions and profile-specific scopes; it is not one current-engine certificate or a combined-corpus result. See [current-frozen-worker-batch-2026-11-01.json](current-frozen-worker-batch-2026-11-01.json).

The F811 seeded-axis query change passed all 32 F811 cases against pinned Ruff and worker `a5a49dec215bf3051ef807e9d9cf85c8438022a6dd7f5f60489981073076283d`; full case output and the old/new query hashes are in [F811-seeded-axis-2026-11-01.json](F811-seeded-axis-2026-11-01.json). The selected Django file list contained no F811 findings under pinned Ruff, so there is no real-corpus F811 positive on which to claim a speedup. A synthetic 300-loop query trace reduced the seeded operator duration from 4.40 ms to 0.93 ms with identical diagnostics; this is a synthetic observation only.

## Engine and harness work

The benchmark request now uses an owned `request.json` file when invoking `run-ryo`, avoiding the reproduced operating-system argument-size limit while preserving exact request bytes. The worker also supports opt-in, flushed JSONL rule/file and query-step diagnostics outside timed execution. Their trace durations are inclusive for nested operators and are not treated as exclusive CPU cost.

Shard resume now handles the crash window where every case row is durable but the shard receipt rename did not happen. It reconstructs an explicit receipt from exact complete case records, marks process exit and stdout summary as unobserved, binds the records hash, and never reruns or fabricates process evidence. The helper self-test passed, including the full-record/missing-receipt regression.

A separate current Django diagnostic run was stopped at its 10-minute cap without a report; it is incomplete and not a benchmark result. The earlier 104-rule six-round measurements remain historical and are described in [historical-report.md](historical-report.md). No performance improvement is claimed from comparing different rule selections, workers, cache modes, or concurrent runs.

## Remaining acceptance work

The active 211-rule cohort still needs a full candidate run on a worker built from the coherent engine/query revision, followed by mismatch classification and repairs. Some authored rule profiles remain explicitly incomplete or unregistered; isolated profile passes do not imply the pinned Ruff inventory has full parity. Remote quality checks and the Rust performance gate also need a terminal successful run before this integration can be considered complete.
