Pinned source: Ruff 0.16.5 `ruff/rules/unnecessary_key_check.rs` and its AST checker dispatch. This query checks a two-element And expression only in Ruff boolean-test contexts, with one In comparison followed by a matching subscript. Structural expression identity uses the existing `expression-shape` relation rather than source text.

`boolean-test-context` follows statement tests, conditional-expression tests, comprehension filters, match guards, logical-not operands, and the first positional argument of the builtin bool call. Boolean operators propagate a boolean-test flag; arbitrary intervening value expressions clear it. `in-comparison` requires one comparator and distinguishes In from NotIn.

The existing `without-effect` helper rejects SideEffect::Present. Possible f-string interpolation effects remain eligible, matching this rule's diagnostic policy; fix applicability is separate. No dictionary type inference is required by the upstream rule.

All 38 controls pass on source-bound worker 2ea with shared suppression 72ea, including the exact tagged upstream fixture. Full 413-rule reference reports are retained in `full-cases.toml`; `cases.toml` is the explicit standalone projection. The receipt `results/ruf019-2026-11-01.json`, pruned after `a6e880e1d7`, binds actual queries, module registry, cases, and paired records. This is scoped evidence, not a full parity claim.

### Lambda evaluation phases

The shared `without-effect` predicate examines effects performed while evaluating the supplied expression. Lambda defaults run when the function object is created; its body runs when called. A lambda body is therefore a separate evaluation region, while defaults remain in the enclosing region, including nested lambdas. `evaluated-targets` is only a syntactic lambda-phase filter: conditional and short-circuit branches remain conservatively included, and generator handling is unchanged. This does not establish value equality, allocation identity, or purity when the function is invoked. The rule retains its existing separate structural and applicability checks.

The phase controls cover body calls/subscripts, positional and keyword-only defaults, deferred and eager nested defaults, and a lambda body whose root is itself an effect node. UP032 also checks seeds already inside a lambda body. Existing source fixtures remain intact; FURB132/RUF019 deferred-body expectations reflect this phase distinction. UP032 preserves its prior conservative repeat policy separately: its lexical call veto and independently checked lambda bodies can reject repetition even when creation has no immediate effect. No new allocation-policy exceptions are introduced; single-use arguments retain their prior policy. Candidate and reference regression receipts are pending for this source batch.

For original fixtures with an undeclared container, the independent runtime check supplies an explicitly documented dictionary namespace; it does not infer a static container type. Separate `explicit-dict-*` controls declare that prerequisite in the source itself. This establishes the scoped lint analysis, not a universally safe rewrite for arbitrary mapping protocols.

Basis: [Python 3.10 lambda expressions](https://docs.python.org/3.10/reference/expressions.html#lambda) and [function default evaluation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions).

### Exact lambda-phase reference differences

The cases below retain their original source bodies in `cases.toml`. Pinned Ruff 0.16.5 reports no finding; the intended finding uses creation-time effects under this rule's built-in-container contract. This is a case-specific analysis difference, not a claim that an arbitrary custom protocol or automatic rewrite is equivalent.

```python
if (lambda:make()) in data and data[lambda:make()]: ...
```

| Case | Intended full finding | Pinned Ruff |
|---|---|---|
| `ruf019-lambda-effect` | `RUF019` bytes `3..50` | none |
| `lambda-phase-body-call` | `RUF019` bytes `3..58` | none |
| `lambda-phase-body-subscript` | `RUF019` bytes `3..60` | none |
| `lambda-phase-nested-deferred-default` | `RUF019` bytes `3..86` | none |
| `lambda-phase-nested-body-root` | `RUF019` bytes `3..74` | none |
| `explicit-dict-ruf019-lambda-effect` | `RUF019` bytes `13..60` | none |
| `explicit-dict-lambda-phase-body-call` | `RUF019` bytes `13..68` | none |
| `explicit-dict-lambda-phase-body-subscript` | `RUF019` bytes `13..70` | none |
| `explicit-dict-lambda-phase-nested-body-root` | `RUF019` bytes `13..84` | none |
| `explicit-dict-lambda-phase-nested-deferred-default` | `RUF019` bytes `13..96` | none |

The [independent CPython 3.10.19 run](../../profiles/RUF019/evidence/lambda-phase-2026-11-03/runtime.json) derives byte spans from stdlib AST and evaluates every exact source plus a scoped `discard`/`get` control. Eligibility rests on zero observed creation effects in these closed inputs. The empty-container comparison alone proves neither eligibility nor universal rewrite safety. Its [inputs](../../profiles/RUF019/evidence/lambda-phase-2026-11-03/inputs.json) contain no expected or candidate arrays; the [re-runnable program](../../profiles/RUF019/evidence/lambda-phase-2026-11-03/program.py) also checks eager-default negative controls. The [contract](../../profiles/RUF019/evidence/lambda-phase-2026-11-03/contract.json) states namespace assumptions and binds the pinned tool, target, settings and current profile. Raw current-profile observations are retained separately from strict certified replay.

Original undeclared-container cases explicitly receive a dictionary only in the independent runtime namespace; no static type knowledge is inferred. Five separate `explicit-dict-*` controls declare the prerequisite in source; their raw observations are also retained.

No upstream issue was opened. Normal shared-harness certificate validation and source review remain required before accepting this source batch.
