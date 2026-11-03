# UP032 source candidate

The query uses existing semantic syntax, call arguments, brace format fields, and the shared effect predicate. It qualifies the brace parse field check and groups arguments by call before joining, so multiple arguments do not multiply findings.

- All four [pinned Ruff 0.16.5 upstream fixtures](https://github.com/astral-sh/ruff/tree/0.16.5/crates/ruff_linter/resources/test/fixtures/pyupgrade) are embedded intact in `cases.toml`: 93 exact UP032 findings.
- Twenty-five independent controls cover positional and keyword fields, repeated pure/effectful arguments, lambda bodies and defaults, missing and nested fields, starred and unpacked arguments, string arguments, suppression, two calls, and two files: 10 exact findings.
- Python target 3.10: [original validation](../../results/up032-source-validation-2026-11-03/validation.json) preserves the first 23/23 receipts; [effect-control validation](../../results/up032-source-validation-2026-11-03/effect-controls-validation.json) records 29/29 candidate/reference raw parity on full diagnostic reports from the same sealed worker. A scoped Fable review proposed a lambda-subscript false positive; pinned Ruff and the unchanged query agreed on the added paired controls.

This is a source candidate until complete review and an exact-head full inventory run. The Python 3.7 probe retained all original cases but found one additional `invalid-syntax` finding in the full upstream fixture on both contestants; the 3.10 expectations do not certify that target. The added controls establish the listed lambda forms, not every possible body or default.
