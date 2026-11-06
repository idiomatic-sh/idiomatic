

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/FURB188/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 2 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/furb188-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `known-empty-suffix-literal-zero`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select FURB188 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (2): `known-empty-suffix-literal-zero`, `known-empty-suffix-literal-len`.

Representative exact source, `known-empty-suffix-literal-zero` (`case.py`):

```python
if text.endswith(""):
    text = text[:-0]
```

- Intended: none. Pinned Ruff observed: FURB188 case.py:0–42.
- Basis: Exact known-empty-string suffix discrepancy: pinned rule documentation promises same-behavior replacement and the actual emitted fix is marked safe. Primary evidence: [exact_python_semantics.py](../../profiles/FURB188/evidence/exact_python_semantics.py), [exact-python-results.json](../../profiles/FURB188/evidence/exact-python-results.json).
- Reproduce: use the [profile](../../profiles/FURB188/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/furb188-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
