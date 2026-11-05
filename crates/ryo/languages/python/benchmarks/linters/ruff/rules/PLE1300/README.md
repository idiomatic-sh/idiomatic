

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLE1300/benchmark.toml)). **Scope:** Language-format semantics versus the pinned lint rule; possible Ruff false positive, not an opened upstream issue.

The 7 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/ple1300-ple1310-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `unicode-decimal-width`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLE1300 --target-version py310 -- case.py
```

### Custom __format__ grammar

Cases (5): `brace-custom-format-policy`, `custom-explicit-index`, `custom-keyword`, `custom-does-not-hide-second-invalid-field`, `assigned-custom-instance`.

Representative exact source, `brace-custom-format-policy` (`case.py`):

```python
class Custom:
    def __format__(self, spec):
        return 'accepted:' + spec
'{:q}'.format(Custom())
```

- Intended: none. Pinned Ruff observed: PLE1300 case.py:80–103.
- Basis: Exact custom-format protocol discrepancy: a locally defined __format__ determines the grammar for this field, so an unsupported standard presentation type alone does not establish an invalid custom format. Primary evidence: [python-format-controls.py](../../profiles/PLE1300/evidence/python-format-controls.py), [python-format-controls-results.json](../../profiles/PLE1300/evidence/python-format-controls-results.json).
- Reproduce: use the [profile](../../profiles/PLE1300/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/ple1300-ple1310-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Unicode decimal width and precision

Cases (2): `unicode-decimal-width`, `unicode-decimal-precision`.

Representative exact source, `unicode-decimal-width` (`case.py`):

```python
'{:٣d}'.format(1)
```

- Intended: none. Pinned Ruff observed: PLE1300 case.py:0–18.
- Basis: Exact Unicode DecimalNumber width/precision discrepancy: actual CPython3.10.19 accepts the configured-target standard format and produces the same result as its independent ASCII decimal control. Primary evidence: [python-format-controls.py](../../profiles/PLE1300/evidence/python-format-controls.py), [python-format-controls-results.json](../../profiles/PLE1300/evidence/python-format-controls-results.json).
- Reproduce: use the [profile](../../profiles/PLE1300/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/ple1300-ple1310-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
