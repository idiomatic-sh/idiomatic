Source-only UP035 controls

Base: e1986beb6e59e20dd20835abe7ad8beea339e991.
Pinned source:
https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/src/rules/pyupgrade/rules/deprecated_import.rs
Full fixture:
https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/resources/test/fixtures/pyupgrade/UP035.py

`pinned-reference.json` binds source bytes and exact normalized findings for all
38 cases across eight targets. Each check used the pinned Ruff executable with
`check --isolated --select UP035 --target-version <target> --output-format json`.
Duplicate findings are compared as multisets, not collapsed by span. Both stub
and Python file suffixes were preserved. No discrepancies or exclusions were
introduced.

The full-inventory registration function staged the canonical singleton and
its current transitive helper closure successfully (identity/closure check
only). No Rust build, query parse/typecheck, or candidate execution was run.

Replay the canonical stage:
`RYO_VERIFY_CASE_RESULTS=<fresh>/cases.jsonl <current-worker> verify-cases <pinned-ruff> /tmp/up035-current-helper-stage-final-20261102/profiles/UP035/repro`

Replay all counter-target stages identically at
`/tmp/up035-target-stages-final-20261102/<py37|py38|py39|py311|py312|py313|py314>`.
Each stage retains 38 cases and the same canonical query/current helper bytes.
The profile target and case expectations are the only target-specific changes.
Actual candidate replay is required before any PASS status update.

Subsequent actual execution evidence

The [UP035/UP036 target supplement](../full-inventory-current/up035-up036-target-supplement/README.md)
now binds corrected current queries to actual sealed-worker execution of every
retained target fixture. Earlier source-only checks and failed executions are
historical, not parser or runtime acceptance proof. The successful sealed input
stages are recorded in that supplement's controls.json and immutable archive;
the original temporary staging paths above retain their earlier draft bytes.
