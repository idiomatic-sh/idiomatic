# UP032 source candidate

The query uses existing semantic syntax, call arguments, brace format fields, and the shared effect predicate. It qualifies the brace parse field check and groups arguments by call before joining, so multiple arguments do not multiply findings.

- All four [pinned Ruff 0.16.5 upstream fixtures](https://github.com/astral-sh/ruff/tree/0.16.5/crates/ruff_linter/resources/test/fixtures/pyupgrade) are embedded intact in `cases.toml`: 93 exact UP032 findings.
- Twenty-six independent controls cover positional and keyword fields, repeated pure/effectful arguments, lambda bodies and defaults, missing and nested fields, starred and unpacked arguments, string arguments, suppression, two calls, two files, and a long repeated name: 12 exact findings.
- Python target 3.10: original validation preserves the first 23/23 receipts; effect-control validation records 29/29 full reports; width-control validation records 30/30 full reports. The review's conditional width concern was checked against the direct pinned Ruff output: both long lines produce UP032. Pinned Ruff and the unchanged query also agreed on the paired lambda controls.

This is a source candidate until complete review and an exact-head full inventory run. The Python 3.7 probe retained all original cases but found one additional `invalid-syntax` finding in the full upstream fixture on both contestants; the 3.10 expectations do not certify that target. The added controls establish the listed lambda forms, not every possible body or default.

### Lambda evaluation phases

The shared `without-effect` predicate examines effects performed while evaluating the supplied expression. Lambda defaults run when the function object is created; its body runs when called. A lambda body is therefore a separate evaluation region, while defaults remain in the enclosing region, including nested lambdas. `evaluated-targets` is only a syntactic lambda-phase filter: conditional and short-circuit branches remain conservatively included, and generator handling is unchanged. This does not establish value equality, allocation identity, or purity when the function is invoked. The rule retains its existing separate structural and applicability checks.

The phase controls cover body calls/subscripts, positional and keyword-only defaults, deferred and eager nested defaults, and a lambda body whose root is itself an effect node. UP032 also checks seeds already inside a lambda body. Existing source fixtures remain intact; FURB132/RUF019 deferred-body expectations reflect this phase distinction. UP032 preserves its prior conservative repeat policy separately: its lexical call veto and independently checked lambda bodies can reject repetition even when creation has no immediate effect. No new allocation-policy exceptions are introduced; single-use arguments retain their prior policy. Candidate and reference regression receipts are pending for this source batch.

Basis: [Python 3.10 lambda expressions](https://docs.python.org/3.10/reference/expressions.html#lambda) and [function default evaluation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions).

### Pinned fix observation outside diagnostic parity

For `"{0!r}|{0!r}".format(lambda: 0)`, pinned Ruff 0.16.5 offers a JSON edit marked `safe`: `f"{lambda: 0!r}|{lambda: 0!r}"`. Compiling that offered edit on CPython 3.10.19 raises `SyntaxError`; Ruff's `--fix-only` invocation retained the original source. The raw JSON edit and tool/target identities, offered-edit compile results, and fix stderr preserve the exact observation.

This does not create a diagnostic discrepancy or an allocation-identity claim: Ryo implements no automatic fix, and UP032's prior diagnostic acceptance policy is preserved. The separate code-object probe compiled successfully and did not establish distinct-field identity behavior. No upstream issue was opened.
