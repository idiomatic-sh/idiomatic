# idiomatic-sh-ryo-benchmark

Shared development harness for comparing Ryo with pinned linter defaults.

This is an unfinished pilot, not a correctness or performance result. Ruff 0.16.5 ships 413 selected rules. One E722 query is authored, with known suppression differences; F401 and F821 have independently verified draft cases but no queries. The audit refuses measurement while any default rule is missing. Tastemaker architecture findings remain open. Do not treat this branch as merge-ready.

From `crates/ryo`, build the Python runner with `cargo build -p idiomatic-sh-ryo-python --bench ruff`. Its executable is `target/debug/deps/ruff-<hash>`; Cargo prints the exact path with `--message-format=json`. The runner accepts `audit`, `verify-tool /absolute/path/to/ruff`, `verify-cases /absolute/path/to/ruff`, and `measure /absolute/path/to/ruff /absolute/path/to/fresh-cache`. Cargo's benchmark harness is disabled; invoke the executable directly so these arguments reach it unchanged.

The profile pins the Ruff executable hash, version, and tagged upstream default selectors. Projects and reversible patches belong to Python's benchmark directory, shared by future Python contestants. Each linter owns its command, output schema, settings, and RyoQL rules. Test cases use UTF-8 byte ranges. Missing tools, incomplete coverage, operational errors, unknowns, unread files and diagnostic differences are errors, never empty success.

The shared process timer measures elapsed child wall time, including startup, SDK/query loading, compilation and output serialization. Decoding happens after the timer. CPU time and peak memory are not implemented; no CPU/RSS comparison or Asymptote ratchet is claimed. Cold, warm and edited runs are gated by exact diagnostic multisets. Complete default coverage and case verification are prerequisites for the measure command.

## Domains

### diagnostics

Canonical finding multisets retain duplicate occurrences, exact paths and byte locations.

### measurements

Process observations separate measured wall time from decoded reports.

### suite

Pinned profiles, authored cases and shared project inputs define the benchmark contract.
