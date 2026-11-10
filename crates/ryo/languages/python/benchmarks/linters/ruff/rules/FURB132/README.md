Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`; target Python 3.10, stable default settings. All prior expected arrays remain unchanged, including the formerly failing recursive-initializer examples and complete tagged upstream fixture.

The effect guard follows `contains_effect`, including literal-only binary operands, effectful descendants, empty builtin initializers, and possible versus present f-string formatting effects. The membership test has one operator. Shared binding typing preserves annotation precedence, parameters, walrus/with initializers, return annotations, typing aliases and prior-binding checks.

Validation: 101/101 exact cases pass on actual benchmark worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Each case has complete reference/candidate records and zero unknown rule/file pairs. This bounded evidence is not a whole-corpus parity certificate. Receipts/hashes: `assigned-type-20261101/verification.json`. No prepared-manifest registration changes.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/FURB132/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `function-return-annotation`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select FURB132 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `function-return-annotation`.

Representative exact source, `function-return-annotation` (`case.py`):

```python
def out()->set:
    pass
if item in out:
    out.remove(item)
```

- Intended: none. Pinned Ruff observed: FURB132 case.py:25–61.
- Basis: This binding denotes the function object itself, not a call result. Primary evidence: [function-object-probe.py](../../profiles/FURB132/evidence/function-object-probe.py), [function-object-inputs.json](../../profiles/FURB132/evidence/function-object-inputs.json).
- Reproduce: use the [profile](../../profiles/FURB132/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

## Shared context-target and value-inference audit

These are exact diagnostic-contract differences, not a blanket waiver for invalid programs. A diagnostic requiring a known receiver type cannot use the context manager as evidence of the enter result. All seven protocol cases use builtin dict/set/list managers with no enter/aenter method; no legitimate target type is lost. Actual Python3.10/3.13 controls cover every original source, valid self-return and different-return managers, and known StringIO self-return. Unknown custom managers remain uninferred.

### with-initializer

```python
with set() as s:
    if x in s:
        s.remove(x)
```

Pinned Ruff0.16.5: FURB132 21..51. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

### with-initializer-extra

```python
with set() as out:
    if item in out:
        out.remove(item)
```

Pinned Ruff0.16.5: FURB132 23..63. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

Primary basis: [with-statement result assignment](https://docs.python.org/3.10/reference/compound_stmts.html#the-with-statement), [IOBase context protocol](https://docs.python.org/3.10/library/io.html#io.IOBase), [function creation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions), and [annotation/runtime distinction](https://docs.python.org/3.10/library/typing.html#typing.get_type_hints).

The exact source dictionaries, complete observed/intended arrays and declared execution environments are retained in cases.toml certificates. [Actual protocol controls](../../profiles/FURB132/evidence/context-target-protocol-py310.json), [valid manager controls](../../profiles/FURB132/evidence/context-target-valid-context-py310.json), [value/annotation controls](../../profiles/FURB132/evidence/context-target-values-py310.json) and [shared semantic contract](../../profiles/FURB132/evidence/context-target-contract.json) are source bound. Python3.13 controls cover supported generic syntax; Python3.10 retains unrelated unsupported-header findings. No upstream issue was filed.

Targeted current-source proof and raw archive retain every original fixture source and complete affected-consumer replays. A fresh shared full413 baseline is required before publication; historical receipts are unchanged.

### Lambda evaluation phases

The shared `without-effect` predicate examines effects performed while evaluating the supplied expression. Lambda defaults run when the function object is created; its body runs when called. A lambda body is therefore a separate evaluation region, while defaults remain in the enclosing region, including nested lambdas. `evaluated-targets` is only a syntactic lambda-phase filter: conditional and short-circuit branches remain conservatively included, and generator handling is unchanged. This does not establish value equality, allocation identity, or purity when the function is invoked. The rule retains its existing separate structural and applicability checks.

The phase controls cover body calls/subscripts, positional and keyword-only defaults, deferred and eager nested defaults, and a lambda body whose root is itself an effect node. UP032 also checks seeds already inside a lambda body. Existing source fixtures remain intact; FURB132/RUF019 deferred-body expectations reflect this phase distinction. UP032 preserves its prior conservative repeat policy separately: its lexical call veto and independently checked lambda bodies can reject repetition even when creation has no immediate effect. No new allocation-policy exceptions are introduced; single-use arguments retain their prior policy. Candidate and reference regression receipts are pending for this source batch.

Basis: [Python 3.10 lambda expressions](https://docs.python.org/3.10/reference/expressions.html#lambda) and [function default evaluation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions).

### Exact lambda-phase reference differences

The cases below retain their original source bodies in `cases.toml`. Pinned Ruff 0.16.5 reports no finding; the intended finding uses creation-time effects under this rule's built-in-container contract. This is a case-specific analysis difference, not a claim that an arbitrary custom protocol or automatic rewrite is equivalent.

```python
out=set()
if (lambda: factory()) in out:
    out.remove((lambda: factory()))
```

| Case | Intended full finding | Pinned Ruff |
|---|---|---|
| `effect-policy-12` | `FURB132` bytes `10..76` | none |
| `lambda-phase-body-call` | `FURB132` bytes `10..74` | none |
| `lambda-phase-body-subscript` | `FURB132` bytes `10..76` | none |
| `lambda-phase-nested-deferred-default` | `FURB132` bytes `10..102` | none |
| `lambda-phase-nested-body-root` | `FURB132` bytes `10..90` | none |

The [independent CPython 3.10.19 run](../../profiles/FURB132/evidence/lambda-phase-2026-11-03/runtime.json) derives byte spans from stdlib AST and evaluates every exact source plus a scoped `discard`/`get` control. Eligibility rests on zero observed creation effects in these closed inputs. The empty-container comparison alone proves neither eligibility nor universal rewrite safety. Its [inputs](../../profiles/FURB132/evidence/lambda-phase-2026-11-03/inputs.json) contain no expected or candidate arrays; the [re-runnable program](../../profiles/FURB132/evidence/lambda-phase-2026-11-03/program.py) also checks eager-default negative controls. The [contract](../../profiles/FURB132/evidence/lambda-phase-2026-11-03/contract.json) states namespace assumptions and binds the pinned tool, target, settings and current profile. Raw current-profile observations are retained separately from strict certified replay.

The three existing context/annotation discrepancies retain their original language evidence and expected arrays. Only their module-profile identity is refreshed against the new current-profile observation; historical execution is not relabelled.

No upstream issue was opened. Normal shared-harness certificate validation and source review remain required before accepting this source batch.
