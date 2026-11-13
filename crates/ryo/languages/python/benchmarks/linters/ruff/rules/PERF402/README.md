Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`; target Python 3.10, stable default settings. All prior expected arrays remain unchanged, including the formerly failing recursive-initializer examples and complete tagged upstream fixture.

Single expression-statement bodies, name targets/arguments, append-or-insert spelling (including runtime-invalid one-argument insert), keyword exclusion and async exclusion follow upstream. Shared binding typing now handles recursive initializers while preserving annotations and only-binding policy.

Validation: 65/65 exact cases pass on actual benchmark worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Each case has complete reference/candidate records and zero unknown rule/file pairs. This bounded evidence is not a whole-corpus parity certificate. Receipts/hashes: `assigned-type-20261101/verification.json`. No prepared-manifest registration changes.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PERF402/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `function-return-annotation`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PERF402 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `function-return-annotation`.

Representative exact source, `function-return-annotation` (`case.py`):

```python
def out()->list:
    pass
for item in xs:
    out.append(item)
```

- Intended: none. Pinned Ruff observed: PERF402 case.py:46–62.
- Basis: This binding denotes the function object itself, not a call result. Primary evidence: [function-object-probe.py](../../profiles/PERF402/evidence/function-object-probe.py), [function-object-inputs.json](../../profiles/PERF402/evidence/function-object-inputs.json).
- Reproduce: use the [profile](../../profiles/PERF402/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

## Shared context-target and value-inference audit

These are exact diagnostic-contract differences, not a blanket waiver for invalid programs. A diagnostic requiring a known receiver type cannot use the context manager as evidence of the enter result. All seven protocol cases use builtin dict/set/list managers with no enter/aenter method; no legitimate target type is lost. Actual Python3.10/3.13 controls cover every original source, valid self-return and different-return managers, and known StringIO self-return. Unknown custom managers remain uninferred.

### with-initializer-extra

```python
with list() as out:
    for item in xs:
        out.append(item)
```

Pinned Ruff0.16.5: PERF402 48..64. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

Primary basis: [with-statement result assignment](https://docs.python.org/3.10/reference/compound_stmts.html#the-with-statement), [IOBase context protocol](https://docs.python.org/3.10/library/io.html#io.IOBase), [function creation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions), and [annotation/runtime distinction](https://docs.python.org/3.10/library/typing.html#typing.get_type_hints).

The exact source dictionaries, complete observed/intended arrays and declared execution environments are retained in cases.toml certificates. [Actual protocol controls](../../profiles/PERF402/evidence/context-target-protocol-py310.json), [valid manager controls](../../profiles/PERF402/evidence/context-target-valid-context-py310.json), [value/annotation controls](../../profiles/PERF402/evidence/context-target-values-py310.json) and [shared semantic contract](../../profiles/PERF402/evidence/context-target-contract.json) are source bound. Python3.13 controls cover supported generic syntax; Python3.10 retains unrelated unsupported-header findings. No upstream issue was filed.

Targeted current-source proof and raw archive retain every original fixture source and complete affected-consumer replays. A fresh shared full413 baseline is required before publication; historical receipts are unchanged.
