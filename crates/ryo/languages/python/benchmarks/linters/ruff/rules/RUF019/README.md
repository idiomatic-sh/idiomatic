Pinned source: Ruff 0.16.5 `ruff/rules/unnecessary_key_check.rs` and its AST checker dispatch. This query checks a two-element And expression only in Ruff boolean-test contexts, with one In comparison followed by a matching subscript. Structural expression identity uses the existing `expression-shape` relation rather than source text.

`boolean-test-context` follows statement tests, conditional-expression tests, comprehension filters, match guards, logical-not operands, and the first positional argument of the builtin bool call. Boolean operators propagate a boolean-test flag; arbitrary intervening value expressions clear it. `in-comparison` requires one comparator and distinguishes In from NotIn.

The existing `without-effect` helper rejects SideEffect::Present. Possible f-string interpolation effects remain eligible, matching this rule's diagnostic policy; fix applicability is separate. No dictionary type inference is required by the upstream rule.

All 38 controls pass on source-bound worker 2ea with shared suppression 72ea, including the exact tagged upstream fixture. Full 413-rule reference reports are retained in `full-cases.toml`; `cases.toml` is the explicit standalone projection. The receipt in `results/ruf019-2026-11-01.json` binds actual queries, module registry, cases, and paired records. This is scoped evidence, not a full parity claim.
