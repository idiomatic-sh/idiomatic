# F821 binding and deferred-read policy

The rule retains its original 61 controls and adds the 21 exact sources from the immutable 310-rule cross-profile run, plus 48 negative and positive controls for deferred reads, deletion, type parameters, and annotation timing. `cross-profile-cases.toml` retains the complete 310-rule expected arrays; the rule profile selects F821 and parser diagnostics explicitly. Passing this profile is evidence for this rule, not a claim that the entire cross-profile report passes.

The query follows Ruff 0.16.5 checker namespace behavior. A first module-level deletion can materialize an implicit builtin; a function-local deletion cannot. Conditional, exception-handler, and else-branch deletion policy follows `handle_node_delete`. Type-parameter bindings belong to their lexical type scope rather than the containing value namespace. Their visibility includes bounds, annotations, class headers, and bodies, and excludes their own defaults and decorators; closer function bindings or deletions still shadow them.

`lookup-read-events` specializes the shared namespace resolver for read semantics, including first stub-only annotation bindings. Later annotations do not overwrite an existing namespace binding. Deferred lookup prefers bound module declarations, permits stub annotation declarations, and restores only the immediately preceding bound declaration for a forward reference preceding a stub deletion. Annotation-only declarations and deletions remain unbound for restoration.

`eager-before-future` compensates for frontend-wide annotation deferral when a future import occurs late: earlier function headers and module/class annotations use a checkpoint preceding their actual evaluation successor. Checkpoints are real event ordinals; no fabricated nodes or spans are used. Function-local annotations retain checker analysis timing.

Actual frozen benchmark-worker evidence: 130 cases, 260 complete reference/candidate records, zero unknown or unread outcomes, worker SHA `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`; receipt `f821-sol-seven/cases.jsonl`. Shared metaclass consumers were separately checked after the resolver output gained its resolved scope column: PYI019 139, PYI034 72, PIE790 12, PLR1708 17, and PLC0206 42 cases.

The final immutable 310-rule run later grew to 72 affected F821 cases (329 extra findings). The additional 51 distinct sources are now retained in the canonical cases and complete cross-profile arrays. A candidate-only replay of those 51 sources selected all 310 frozen queries with the F821 fix, paired against the original sealed reference records: 102 complete records, zero unknown/unread pairs, zero skipped pairs, and zero F821 differences. Of the complete reports, 46 match exactly; the five remaining differences are four F811 missing findings and one F841 extra finding, retained in `f821-final-full310-sol/result.json`. This follow-up does not claim those other rules pass. No reference reruns were used for this replay.

Scoped future follow-up: the canonical union now has 207 cases. `future-context-full-cases.toml` retains the full 310-rule reports for additional ordered/nested future controls, including the frontend owner's eight regression sources. Existing sources reuse sealed reference records; 25 genuinely new sources were checked once against the pinned full 310-rule reference selection. Shared helpers now follow statement/exception restoration, inherited definition snapshots, whole annotation-root deferral, runtime PEP 613/TypeVar boundaries, and skipped operands. Old-worker candidate-only evidence under `future-scoped-policy-oldworker-sol/F821/` has complete coverage and zero differences for this rule; nine cases retain only the known frontend syntax-validation differences (early, nested, relative, or lazy future behavior). Final whole-profile pass status awaits the frontend owner's fresh-worker replay. Every new invocation persists its exact request, profile, module registry, source input, and raw response; historical missing request metadata was not reconstructed as if recorded.

## Current validity and binding discrepancy audit

The historical replay descriptions above are not the current status. All 207 original case sources are retained. These exact discrepancies are staged for strict replay and peer review; this source audit alone is not a PASS.

Each reproducer, original full reference array, independently intended full array, and case-specific rationale is preserved in [the exact-source audit](../../profiles/F821-stub-annotation-context/evidence/current-independent-full-intended.json). The reference is pinned Ruff 0.16.5; no upstream issue was opened. Grammar/compiler validation is distinct from portable stub typing, and future-placement/unknown-feature diagnostics retain the declared F404/F407 namespaces. These are documented semantics or reporting-policy differences, not a blanket claim that Ruff is wrong on invalid programs.

The relative-future case explicitly preserves CPython 3.10.19's historical activation behavior as contrary evidence. [CPython issue 118216](https://github.com/python/cpython/issues/118216) and [merged correction 118267](https://github.com/python/cpython/pull/118267) establish the corrected zero-leading-dot directive contract; the independently recorded 3.13.5 control agrees with that correction. Target 3.10 is not claimed to agree.

| Exact case/reproducer | Observed pinned reference | Intended full diagnostics |
| --- | --- | --- |
| `cross-PYI034-metaclass-function-annotation-typescope` | F821 77..84, invalid-syntax 74..84 | F821 77..84 |
| `future-scope-class-forward-type` | none | F821 59..64 |
| `future-scope-class-scoped-import` | invalid-syntax 55..68 | none |
| `future-scope-early-definition` | none | invalid-syntax 50..55 |
| `future-scope-except-restoration` | F821 121..128 | F821 121..128, F821 77..83 |
| `future-scope-function-forward-type` | none | F821 63..68 |
| `future-scope-function-scoped-import` | invalid-syntax 63..76 | none |
| `future-scope-if-else-inheritance` | F821 3..7 | F821 3..7, F821 61..66 |
| `future-scope-if-restoration` | F821 3..7, F821 103..110 | F821 3..7, F821 103..110, F821 59..65 |
| `future-scope-later-definition` | invalid-syntax 83..96 | none |
| `future-scope-nested-future-blocks-type-scope-walrus` | invalid-syntax 59..72 | invalid-syntax 102..107 |
| `future-scope-relative-future-flag` | invalid-syntax 45..58 | none |
| `future-scope-stub-remains-postponed` | invalid-syntax 9..22 | none |
| `future-scope-try-handler-inheritance` | none | F821 69..74 |
| `stub-named-expression-preserves-undefined-read` | invalid-syntax 10..16, F821 15..16 | F821 15..16 |

Ranges use independent original token/AST positions, not candidate output. Source-identical compiler originals and valid counterparts are linked in the evidence directory. Six F821 eager-lookup cases additionally retain both unbound and bound runtime controls with unchanged lexical lookup timing; the PLW0602 eager-annotation case preserves its live outer-binding control. No unrelated lint finding is removed. Compiler first-error output establishes invalidity where applicable, not our exhaustive diagnostic count or namespace policy.
