UP036 source-only controls

Base: e1986beb6e59e20dd20835abe7ad8beea339e991.
Pinned source:
https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/src/rules/pyupgrade/rules/outdated_version_block.rs
All six fixtures are from the same commit, `resources/test/fixtures/pyupgrade/UP036_{0..5}.py`.
Their complete source bytes remain in every target's case suite.

`pinned-reference.json` records exact source hashes and original reference
findings for 114 cases (38 per target). `micro-pinned-settings.json` additionally
captures the three discrepancy observations under the benchmark's exact
normalized CLI settings. Case certificates bind those reference findings and
separate intended expectations to their profile, source, settings, approved
reference executable, and independent evidence artifacts.

Reproduce reference controls:
`python3 check_reference.py /home/vscode/.local/bin/ruff`
Reproduce the independent AST policy audit:
`python3 audit_policy.py`
These source/reference checks are not candidate execution evidence.

Each target's profile evidence executes the complete unchanged micro-version
fixture against explicitly modeled (3, minor, 0) and (3, minor, 3) tuples.
The micro predicate takes both branches while the major-index positive control
always takes the major-three branch. These are modeled supported patch values;
the JSON separately records the real host interpreter version. No claim is
made to have run those historical interpreter patch releases.

Canonical current-helper replay stage:
`/tmp/up036-current-helper-final-20261102/profiles/UP036/repro`
Counter-target stages:
`/tmp/up036-target-stages-final-20261102/py37`
`/tmp/up036-target-stages-final-20261102/py313`
Run each with:
`RYO_VERIFY_CASE_RESULTS=<fresh>/cases.jsonl <current-worker> verify-cases <pinned-ruff> <stage>`

No Rust build, parser/typecheck, or candidate replay was performed. All profiles
retain 38 cases. UP036 remains FAIL until actual candidate proof.

Subsequent actual execution evidence

The [UP035/UP036 target supplement](../full-inventory-current/up035-up036-target-supplement/README.md)
now binds corrected current queries to actual sealed-worker execution of every
retained target fixture. Earlier source-only checks and failed executions are
historical, not parser or runtime acceptance proof. The successful sealed input
stages are recorded in that supplement's controls.json and immutable archive;
the original temporary staging paths above retain their earlier draft bytes.
