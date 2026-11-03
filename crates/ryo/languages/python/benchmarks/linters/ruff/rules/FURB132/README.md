Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`; target Python 3.10, stable default settings. All prior expected arrays remain unchanged, including the formerly failing recursive-initializer examples and complete tagged upstream fixture.

The effect guard follows `contains_effect`, including literal-only binary operands, effectful descendants, empty builtin initializers, and possible versus present f-string formatting effects. The membership test has one operator. Shared binding typing preserves annotation precedence, parameters, walrus/with initializers, return annotations, typing aliases and prior-binding checks.

Validation: 101/101 exact cases pass on actual benchmark worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Each case has complete reference/candidate records and zero unknown rule/file pairs. This bounded evidence is not a whole-corpus parity certificate. Receipts/hashes: `/tmp/assigned-type-20261101/verification.json`. No prepared-manifest registration changes.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/FURB132/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `function-return-annotation`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select FURB132 --target-version py310 -- case.py
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
