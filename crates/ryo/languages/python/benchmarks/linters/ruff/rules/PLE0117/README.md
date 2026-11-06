

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLE0117/benchmark.toml)). **Scope:** Compiler-invalid source diagnostic omitted by the pinned reference; no upstream issue was opened.

The 4 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/ci-80be-full-inventory-2026-11-03/ple0115-ple0117-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `crossrule-PLE0115-global-then-nonlocal`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLE0117 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (4): `own-global-conflict-with-real-outer-binding`, `crossrule-PLE0115-global-then-nonlocal`, `crossrule-PLE0115-noqa-suppresses-conflicting-declaration`, `crossrule-PLE0115-each-nonlocal-declaration-retains-diagnostic-multiplicity`.

Representative exact source, `crossrule-PLE0115-global-then-nonlocal` (`case.py`):

```python
def f():
    global value
    nonlocal value
```

- Intended: PLE0117 case.py:39–44; invalid-syntax case.py:39–44. Pinned Ruff observed: PLE0117 case.py:39–44.
- Basis: Exact reference omission of compiler-validity diagnostics: a name cannot be both global and nonlocal in the same scope. Primary evidence: [compile-controls.py](../../profiles/PLE0117/evidence/compile-controls.py), [compile-controls-inputs.json](../../profiles/PLE0117/evidence/compile-controls-inputs.json).
- Reproduce: use the [profile](../../profiles/PLE0117/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/ci-80be-full-inventory-2026-11-03/ple0115-ple0117-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
