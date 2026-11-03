

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLE1700/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `authored-class-base-outer-async`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLE1700 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `authored-class-base-outer-async`.

Representative exact source, `authored-class-base-outer-async` (`case.py`):

```python
async def f():
    class C((yield from xs)): pass
```

- Intended: PLE1700 case.py:28–41. Pinned Ruff observed: none.
- Basis: The pinned reference omits an invalid await/yield-from diagnostic that the configured Python language independently rejects in this exact lexical context. Primary evidence: [cpython-3.10.19.json](../../profiles/PLE1700/evidence/cpython-3.10.19.json), [cpython-3.13.5.json](../../profiles/PLE1700/evidence/cpython-3.13.5.json).
- Reproduce: use the [profile](../../profiles/PLE1700/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
