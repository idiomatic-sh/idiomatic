# Ruff benchmark integration status

**In progress.** The target remains all 413 Ruff 0.16.5 defaults as RyoQL. Full correctness and current performance gates have not passed.

## Frozen combined run

The active acceptance descriptor is [current-canonical-ready-310-r2](../profiles/current-canonical-ready-310-r2/benchmark.toml): 310 selected PY310 rules, 6,553 cases, and 13,526 pinned reference findings. Its [reference provenance](../profiles/current-canonical-ready-310-r2/reference-provenance.json) proves exact reuse of the prior 6,542 reference records and pinned Ruff invocations for only 11 new F841 cases. Query-only corrections did not rerun the reference corpus. The original 6,542-case raw oracle remains immutable and archived.

The [READY asset receipt](../profiles/current-canonical-ready-310-r2/ready-assets.json) binds exact candidate/query and dependency module bytes, including source commit hashes for imported READY fixes. Every previous case object is retained; historical descriptors use separate archived input bytes. Candidate inputs are staged outside the source tree, and worker reuse preserves the executable's original stable build identity.

The candidate run uses worker SHA-256 `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b` and 32 resumable shards. A deliberate resource correction changed scheduling from one to four concurrent workers while preserving its manifest, worker, staged inputs, and complete case records. Linux memory sizing now reads `MemAvailable`; `--workers` provides an explicit bounded override.

The run is unfinished and has reproduced cross-rule differences. Isolated profile passes and query-delta replays are scoped evidence, not a combined parity certificate. PLE0118's corrected query removes its findings in three retained B019 cross-cases; two full reports match, while the third retains an independent F811 difference.

## Next source cohort

[The machine-derived inventory](source-inventory-2026-11-01.json) records 328 of 413 defaults with query files in this tree and 85 without query files. File presence does not establish a correct implementation. The next prepared PY310 selection spans 319 defaults; PY311-specific profiles remain separate.

[Eight READY source batches](next-ready-source-batch-2026-11-01.json) integrate method, typing, iteration, call policy, string, conditional, and collection query work. Candidate/query files, declared dependencies, and prior case unions were imported together. The root-restricted `assigned-values` implementation is retained. These later source changes do not alter the frozen 310-rule run.

## Compiler and quality evidence

The compiler checkpoint passes 386 local tests: 222 core unit tests, 150 Python unit tests, and 14 Ruff harness tests. The new parser regression covers comprehension-variable rebinding, nested comprehensions, lambda body/default scope, destructured targets, and retained accepted AST facts. Denied enum/cast errors from all three old CI platforms and additional DFS test casts are fixed.

Focused Clippy completes successfully. Introduced parser, format-reader, relation, and DFS warnings are being removed in source; dependency and benchmark warnings still need review. They are not suppressed or accepted as a new baseline.

CI run `41300226624` failed compilation on Linux, macOS, and Windows at the older public head. Its quality and speed jobs did not establish a pass. A new coherent compiler checkpoint is required before claiming successful remote quality or Rust performance gates.

## Performance

No current corpus speedup is established. The [historical 104-rule measurements](historical-report.md) remain separate: the six-round Django Ryo median was 291.869 seconds versus Ruff's 0.343 seconds. A later diagnostic attempt stopped at its 10-minute cap without a complete lint report. Worker selections, cache modes, and overlapping runs are not interchangeable performance evidence.
