# PLW0602 checker scope and directive policy

The query follows Ruff 0.16.5 current scope bindings and checker analysis timing. Shared namespace/nonlocal fragments distinguish real outer bindings from an invalid own-scope nonlocal, preserve function defaults, and model type-parameter and annotation/class-header type scopes. Deferred reads prefer actual module declarations and materialized builtins; later annotations retain existing bindings. F841 counts actual declaration uses and ignores valid redirected locals. PLW0602 reports only the surviving Global binding, with later assignment/deletion/global/valid-nonlocal effects applied in order.

All 5 prior live-integration controls remain unchanged. The canonical profile has 37 controls, including final frozen cross-rule reproductions and 31 new source-order, nonlocal, builtin, directive-order, and type-scope controls. Complete frozen 310-rule arrays remain in `cross-profile-cases.toml`; full 310-rule arrays for new controls remain in `new-policy-full-cases.toml`. New reference oracles and raw outputs are sealed under `/tmp/nonlocal-policy-oracles-sol/`. Existing reference inputs are reused, not rerun.

Actual old frozen worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b` evaluates every case completely with zero unknown/unread/skipped pairs. All expected PLW0602 findings match. One retained control (`nonlocal-policy-early-annotation-before-late-future`) still has an unrelated provider `invalid-syntax` extra at bytes 9..22 because that worker applies a later future import retroactively. The frontend owner is correcting that source-order validation and will replay these exact arrays with a fresh worker. Until that receipt exists this is a query-policy integration handoff, not a fully passing profile or parity certificate. Old-worker evidence: `/tmp/plw0602-candidate-only-sol/cases.jsonl`.

Scoped future follow-up: the canonical union now has 62 cases. `future-context-full-cases.toml` retains the full 310-rule reports for additional ordered/nested future controls, including the frontend owner's eight regression sources. Existing sources reuse sealed reference records; 25 genuinely new sources were checked once against the pinned full 310-rule reference selection. Shared helpers now follow statement/exception restoration, inherited definition snapshots, whole annotation-root deferral, runtime PEP 613/TypeVar boundaries, and skipped operands. Old-worker candidate-only evidence under `/tmp/future-scoped-policy-oldworker-sol/PLW0602/` has complete coverage and zero differences for this rule; nine cases retain only the known frontend syntax-validation differences (early, nested, relative, or lazy future behavior). Final whole-profile pass status awaits the frontend owner's fresh-worker replay. Every new invocation persists its exact request, profile, module registry, source input, and raw response; historical missing request metadata was not reconstructed as if recorded.

## Current validity and binding discrepancy audit

The historical replay descriptions above are not the current status. All 68 original case sources are retained. These exact discrepancies are staged for strict replay and peer review; this source audit alone is not a PASS.

Each reproducer, original full reference array, independently intended full array, and case-specific rationale is preserved in [the exact-source audit](../../profiles/PLW0602/evidence/current-independent-full-intended.json). The reference is pinned Ruff 0.16.5; no upstream issue was opened. Grammar/compiler validation is distinct from portable stub typing, and future-placement/unknown-feature diagnostics retain the declared F404/F407 namespaces. These are documented semantics or reporting-policy differences, not a blanket claim that Ruff is wrong on invalid programs.

The relative-future case explicitly preserves CPython 3.10.19's historical activation behavior as contrary evidence. [CPython issue 118216](https://github.com/python/cpython/issues/118216) and [merged correction 118267](https://github.com/python/cpython/pull/118267) establish the corrected zero-leading-dot directive contract; the independently recorded 3.13.5 control agrees with that correction. Target 3.10 is not claimed to agree.

| Exact case/reproducer | Observed pinned reference | Intended full diagnostics |
| --- | --- | --- |
| `cross-PLE0117-own-global-conflict-with-real-outer-binding` | none | invalid-syntax 80..85 |
| `future-scope-class-scoped-import` | invalid-syntax 55..68 | none |
| `future-scope-function-scoped-import` | invalid-syntax 63..76 | none |
| `future-scope-later-definition` | invalid-syntax 83..96 | none |
| `future-scope-nested-future-blocks-type-scope-walrus` | PLW0602 85..90, invalid-syntax 59..72 | invalid-syntax 102..107 |
| `future-scope-relative-future-flag` | invalid-syntax 45..58 | none |
| `future-scope-stub-remains-postponed` | invalid-syntax 9..22 | none |
| `nonlocal-policy-annotation-outer` | none | invalid-syntax 60..65 |
| `nonlocal-policy-class-type-parameter` | invalid-syntax 7..10 | invalid-syntax 7..10, invalid-syntax 48..49 |
| `nonlocal-policy-default-real-binding` | none | invalid-syntax 65..70 |
| `nonlocal-policy-delayed-annotation-keeps-outer-value` | none | invalid-syntax 73..78 |
| `nonlocal-policy-deleted-outer` | none | invalid-syntax 69..74 |
| `nonlocal-policy-early-annotation-before-late-future` | none | invalid-syntax 50..55 |
| `nonlocal-policy-future-annotation-not-visible` | PLW0602 69..74, invalid-syntax 44..57 | PLW0602 69..74, invalid-syntax 44..57, invalid-syntax 85..90 |
| `nonlocal-policy-global-no-valid-nonlocal` | PLW0602 17..22 | PLW0602 17..22, invalid-syntax 33..38 |
| `nonlocal-policy-invalid-outer-nonlocal` | PLW0602 48..53 | PLW0602 48..53, invalid-syntax 65..70 |
| `nonlocal-policy-local-stub-class-header-is-late` | PLW0602 65..70 | PLW0602 65..70, invalid-syntax 83..88 |
| `nonlocal-policy-nearer-function-over-type-parameter` | invalid-syntax 9..12 | invalid-syntax 9..12, invalid-syntax 76..77 |
| `nonlocal-policy-nested-function-type-parameter` | invalid-syntax 9..12 | invalid-syntax 9..12, invalid-syntax 48..49 |
| `nonlocal-policy-nonlocal-then-global` | PLW0602 58..63 | PLW0602 58..63, invalid-syntax 43..48 |
| `nonlocal-policy-skip-class-namespace` | none | invalid-syntax 85..90 |
| `nonlocal-policy-type-scope-annotation-py` | none | invalid-syntax 50..55 |
| `nonlocal-policy-type-scope-annotation-pyi` | PLW0602 34..39, invalid-syntax 9..22 | PLW0602 34..39, invalid-syntax 50..55 |
| `nonlocal-policy-type-scope-class-body-py` | none | invalid-syntax 50..55 |
| `nonlocal-policy-type-scope-class-body-pyi` | PLW0602 34..39 | PLW0602 34..39, invalid-syntax 50..55 |
| `nonlocal-policy-type-scope-class-method-py` | none | invalid-syntax 66..71 |
| `nonlocal-policy-type-scope-class-method-pyi` | none | invalid-syntax 66..71 |
| `nonlocal-policy-type-scope-parameter-py` | invalid-syntax 5..8 | invalid-syntax 5..8, invalid-syntax 32..33 |
| `nonlocal-policy-type-scope-parameter-pyi` | invalid-syntax 5..8 | invalid-syntax 5..8, invalid-syntax 32..33 |

Ranges use independent original token/AST positions, not candidate output. Source-identical compiler originals and valid counterparts are linked in the evidence directory. Six F821 eager-lookup cases additionally retain both unbound and bound runtime controls with unchanged lexical lookup timing; the PLW0602 eager-annotation case preserves its live outer-binding control. No unrelated lint finding is removed. Compiler first-error output establishes invalidity where applicable, not our exhaustive diagnostic count or namespace policy.
