

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/FURB122/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 13 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `nested-scope-prebinding`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select FURB122 --target-version py310 -- case.py
```

### IO receiver and annotation

Cases (5): `io-binding-2`, `io-binding-4`, `io-annotation-3`, `independent-nested-generator-does-not-change-factory-result`, `independent-nested-generator-lambda-does-not-change-factory-result`.

Representative exact source, `io-annotation-3` (`case.py`):

```python
import io,typing
def fn(f: typing.IO[str]):
    for x in rows:
        f.write(x)
```

- Intended: FURB122 case.py:48–81. Pinned Ruff observed: none.
- Basis: The declared generic typing.IO[str] has typing.IO as its origin and exposes writelines; the exact framed StringIO consumer writes the same elements. Primary evidence: [qualified-guard-cpython310.json](../../profiles/FURB122/evidence/qualified-guard-cpython310.json).
- Reproduce: use the [profile](../../profiles/FURB122/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Lexical scope and binding history

Cases (3): `nested-scope-prebinding`, `outer-runtime-history-3`, `independent-target-namespace-outer-binding-is-not-function-local-value`.

Representative exact source, `nested-scope-prebinding` (`case.py`):

```python
x=0
def a():
    f=open("a")
    for x in rows:
        f.write(x)
```

- Intended: FURB122 case.py:33–66. Pinned Ruff observed: none.
- Basis: The rule diagnoses the write-loop idiom when its loop target has no prior initialized binding in the same lexical storage namespace and no outside lexical read. Primary evidence: [target-namespace-cpython310.json](../../profiles/FURB122/evidence/target-namespace-cpython310.json), [target-namespace-cpython313.json](../../profiles/FURB122/evidence/target-namespace-cpython313.json).
- Reproduce: use the [profile](../../profiles/FURB122/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### TYPE_CHECKING and runtime shadowing

Cases (5): `runtime-shadow-history-4`, `runtime-shadow-history-5`, `runtime-shadow-history-6`, `rebound-type-checking-true`, `rebound-typing-attribute-true`.

Representative exact source, `runtime-shadow-history-4` (`case.py`):

```python
f=open("x")
if TYPE_CHECKING:
    x=0
for x in rows:
    f.write(x)
```

- Intended: none. Pinned Ruff observed: FURB122 case.py:38–67.
- Basis: The exact source has no import or binding for TYPE_CHECKING; CPython raises NameError at the guard. Primary evidence: [qualified-guard-cpython310.json](../../profiles/FURB122/evidence/qualified-guard-cpython310.json).
- Reproduce: use the [profile](../../profiles/FURB122/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

## Context-result regression controls

The shared helper distinguishes a context manager from its enter result. Existing resolved standard IOBase initialization remains available for synchronous with because that protocol returns the stream itself. Unknown custom manager results remain uninferred; standard IOBase has no asynchronous enter protocol. Valid different-return managers and async StringIO negative controls are retained in cases.toml alongside every original source. [Current targeted evidence](../../results/context-target-inference-2026-11-03/README.md) includes the complete superseding IO replays and actual CPython3.10/3.13 controls. Fresh full413 proof remains required before publication.
