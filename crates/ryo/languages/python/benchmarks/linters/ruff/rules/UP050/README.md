

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/UP050/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `independent-callable-lexical-ownership-0`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select UP050 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `independent-callable-lexical-ownership-0`.

Representative exact source, `independent-callable-lexical-ownership-0` (`case.py`):

```python
def f():
    class C(metaclass=type): pass
    type = object
```

- Intended: none. Pinned Ruff observed: UP050 case.py:21–35.
- Basis: Python compiler-local ownership makes this earlier read unbound; it does not load the outer typing import or builtin. Primary evidence: [lexical-ownership-cpython310.json](../../profiles/UP050/evidence/lexical-ownership-cpython310.json), [lexical-ownership-cpython313.json](../../profiles/UP050/evidence/lexical-ownership-cpython313.json).
- Reproduce: use the [profile](../../profiles/UP050/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
