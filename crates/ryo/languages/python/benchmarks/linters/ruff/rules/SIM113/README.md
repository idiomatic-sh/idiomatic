

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/SIM113/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 2 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `closure-before-initializer`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select SIM113 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (2): `closure-before-initializer`, `closure-inside-loop`.

Representative exact source, `closure-before-initializer` (`case.py`):

```python
def f():
    callback = lambda: i
    i = 0
    for value in values:
        i += 1
    return callback()
```

- Intended: none. Pinned Ruff observed: SIM113 case.py:77–83.
- Basis: The suggested enumerate rewrite changes the returned closure-observed counter from2 to1 for the exact controlled input; retain the semantically required counter. Primary evidence: [counter-exact-cpython-3.10.19.json](../../profiles/SIM113/evidence/counter-exact-cpython-3.10.19.json), [counter-exact-cpython-3.13.5.json](../../profiles/SIM113/evidence/counter-exact-cpython-3.13.5.json).
- Reproduce: use the [profile](../../profiles/SIM113/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
