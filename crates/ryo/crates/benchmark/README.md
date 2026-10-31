# idiomatic-sh-ryo-benchmark

Shared development harness for comparing Ryo with pinned linter defaults.

Ruff 0.16.5 ships 413 selected rules. Each default is tracked as authored, explicitly deferred to an assessed capability issue, or still unassessed. An authored query becomes verified only after both contestants pass its independent cases. Deferrals never count as matches. The full audit refuses unassessed defaults; `measure-scoped` can produce explicitly interim evidence for the authored subset. Tastemaker architecture findings remain open, so this branch is not merge-ready.

From `crates/ryo`, build the Python runner with `cargo build --release -p idiomatic-sh-ryo-python --bench ruff`. Cargo prints the exact executable path with `--message-format=json`. The runner accepts `audit`, `verify-tool /absolute/path/to/ruff`, `verify-cases /absolute/path/to/ruff`, and `measure /absolute/path/to/ruff /absolute/path/to/cache`. Use `measure-scoped` for interim subset evidence while the full inventory still has unassessed defaults. Measurement refuses a debug-profile executable.

The profile pins the Ruff executable hash, version, and tagged upstream default selectors. Projects and reversible patches belong to Python's benchmark directory, shared by future Python contestants. Each linter owns its command, output schema, settings, and RyoQL rules. Test cases use UTF-8 byte ranges. Missing tools, incomplete coverage, operational errors, unknowns, unread files and diagnostic differences are errors, never empty success.

The shared process timer measures elapsed child wall time, including the pinned GNU time collector, startup, SDK/query loading, compilation and output serialization. CPU is the child's user plus system time, including waited-for descendants; GNU time reports seconds with hundredth-second precision. Peak RSS is the OS maximum resident set in KiB, converted to bytes. Decoding, profile/executable certification and source hashing happen outside the timer. BMF stdout reports medians and bounds for Asymptote; stderr retains individual observations and scope evidence. No performance result is implied by the existence of the instrumentation.

Empty-tool-cache, retained-tool-cache and edited runs are gated by exact diagnostic multisets, using six alternating-order trials and one processing worker per contestant. Every trial owns new cache directories; both edited contestants are compared with fresh-cache executions. OS page cache is uncontrolled, and retained caches do not imply verified cache hits. An immutable artifact snapshot seals the exact authored profile, queries, cases and both executable bytes. Every measured pair rechecks that seal and the source bytes. Both contestants receive the same explicit `.py`/`.pyi` files; other default-discovered formats are inventoried as exclusions linked to the frontend capability issue. This is Python-source subset evidence, not full default-discovery parity.

## Domains

### diagnostics

Canonical finding multisets retain duplicate occurrences, exact paths and byte locations.

### measurements

Process observations separate measured wall time from decoded reports.

### suite

Pinned profiles, authored cases and shared project inputs define the benchmark contract.
