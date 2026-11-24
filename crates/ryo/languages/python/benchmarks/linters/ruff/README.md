# ruff benchmark

The Python provider against pinned Ruff 0.16.5. `benchmark.toml` pins the 413 rules Ruff 0.16.5 enables by default; each has a query
in `rules/<id>/` with its cases, and a rule whose cases need settings of their own has its profile in `profiles/<id>/benchmark.toml`.
`profiles/pinned-rule-phases.json` registers each rule's phase and applicability, `profiles/pinned-noqa-code-policy.json` is the
pinned noqa code catalog, and `profiles/pinned-suppression-policy.json` lists the rules whose `codes` argument the catalog renders: the
bench's schema test `suppression_codes_match_the_pinned_catalog` checks every such query, RUF100's catalog lookup and RUF101's
redirected codes against it. The evidence programs and observations a rule's cases declare stay beside their profile or rule, pinned by
digest; they are data the harness stages and hashes, never runs.

**Historical milestone: 413 PASS / 0 FAIL on frozen `3ffb75b3`; this is not the current engine result.** It bound frozen `3ffb75b3`,
its sealed worker and all 11,185 cases. **Current engine:** full run185 recorded 411 PASS / 2 FAIL. On `f0738752d`, targeted run195
passes RUF100 (27/27 cases); B008 still fails its 120-second aggregate timeout. A new full 413-rule pass and full-Django performance
acceptance remain unproven.

The Python authoring tools (`authoring/`, `verify_profiles.py`), the generated `RULES.md`, the `results/` captures (but the UP031
process captures its cases declare, kept under `results/up031-*`), the frozen-manifest profiles (`profiles/prepared.json`,
`native-standalone-*`, `archived-inputs/`, `oracle.json`, the variant and combined profiles), the upstream copies the cases inline and
the evidence no case declares were deleted when the bench moved onto the shared harness; commit `981eb0d869` is the last that holds
them, and `a6e880e1d7` keeps the earlier full-run receipts.

From `crates/ryo`, the shared collector registers every default from `benchmark.toml`, `profiles/<id>/benchmark.toml` and
`rules/<id>/`, builds and seals the worker, verifies every default, then measures ryo against the pinned Ruff over each project
of `../../benchmark.toml`, writing under `target/benchmark/python/ruff/bench/` and printing the bench's one BMF document:

```sh
taskset -c 0-3 env RAYON_NUM_THREADS=4 cargo run --locked --release -p idiomatic-sh-benchmark --bin benchmark-collector -- bench languages/python/benchmarks/linters/ruff
```

That is the asymptote tool `bench/python/ruff`. The rules' growth is `../../growth.toml`. The shared collector owns registration,
execution receipts and PASS/FAIL decisions; missing, incomplete, invalid, or mismatched results fail. `verify-reference-cases` checks
the pinned Ruff observations only; candidate correctness requires `verify-cases` or the full inventory.

EXE001 and EXE002 declare `file-observation = "posix-executable"`: the harness refuses their cases, and a parity or measure run that
selects them, on a host whose file permissions cannot be trusted (a Microsoft kernel outside a container, or a host without POSIX
permissions).

The active pinned phase registry distinguishes the 413 shipped defaults from registered nondefault cohort definitions. Code-to-variant
evidence points to Ruff 0.16.5 `crates/ruff_linter/src/codes.rs`. All 413 existing phase/applicability values are preserved;
supplemental registrations add no inventory credit.

Canonical `measure` and `measure-valid` cold/warm/edit benches use all 413 pinned defaults, with exact rule-ID equality enforced before
staging and again before timed execution. The same immutable cohort configures both contestants. `diagnostic-measure` and
`diagnostic-measure-valid` take the same `--case-inventory=PATH` and may then select a comma-separated rule subset
(`--case-inventory=PATH PLE2510,F401`); they certify only the selected rules' registered cases and emit sample observations but no
headline benchmark metrics.

The canonical corpus profile explicitly declares project search roots `["", "src"]`, matching the pinned isolated Ruff scope and the
reviewed I001 profile. These are host configuration, not inferred engine defaults. Profile bytes and project authority are sealed into
current certification/settings identity.
