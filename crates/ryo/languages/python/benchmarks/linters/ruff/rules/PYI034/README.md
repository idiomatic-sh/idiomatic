

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PYI034/benchmark.toml)). **Scope:** Python grammar acceptance versus restricted stub type-expression policy; this is a scope distinction, not a claim that the stub is portable or well typed.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/ci-80be-full-inventory-2026-11-03/pyi019-pyi034-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `metaclass-function-annotation-typescope`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PYI034 --target-version py310 -- case.pyi
```

### Documented source-specific difference

Cases (1): `metaclass-function-annotation-typescope`.

Representative exact source, `metaclass-function-annotation-typescope` (`case.pyi`):

```python
from typing import TypeVar
T=TypeVar("T")
class A(type): ...
def outer(x:(A:=unknown)):
 class Subject(A):
  def __enter__(self)->Subject: ...
```

- Intended: none. Pinned Ruff observed: invalid-syntax case.pyi:74–84.
- Basis: Ryo records Python grammar as accepted while pinned Ruff reports generic invalid syntax for a named expression in a stub annotation; this does not establish stub typing validity or a Ruff bug. Primary evidence: [audit.py](../../profiles/PYI034/evidence/audit.py), [exact-current-failed-sources.json](../../profiles/PYI034/evidence/exact-current-failed-sources.json).
- Reproduce: use the [profile](../../profiles/PYI034/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/ci-80be-full-inventory-2026-11-03/pyi019-pyi034-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
