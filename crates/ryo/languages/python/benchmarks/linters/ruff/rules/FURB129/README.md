

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/FURB129/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 34 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `binding-8`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select FURB129 --target-version py310 -- case.py
```

### Path or descriptor is not an IO stream

Cases (15): `initializer-3`, `assigned-initializer-3`, `initializer-7`, `assigned-initializer-7`, `initializer-8`, `assigned-initializer-8`, `initializer-9`, `assigned-initializer-9`, `annotation-11`, `annotation-12`, `annotation-13`, `annotation-14`, `annotation-15`, `annotation-16`, `late-annotation-2`.

Representative exact source, `annotation-11` (`case.py`):

```python
import io, os, pathlib, typing, codecs
def f(fp: os.Path):
    for line in fp.readlines():
        pass
```

- Intended: none. Pinned Ruff observed: FURB129 case.py:75–89.
- Basis: CPython 3.10 has no os.Path type; the exact annotation cannot establish a standard IO interface. Primary evidence: [exact-contract-cpython310.json](../../profiles/FURB129/evidence/exact-contract-cpython310.json), [cpython310-io-contract.json](../../profiles/FURB129/evidence/cpython310-io-contract.json).
- Reproduce: use the [profile](../../profiles/FURB129/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Standard IO receiver or annotation

Cases (14): `initializer-10`, `assigned-initializer-10`, `initializer-11`, `assigned-initializer-11`, `annotation-20`, `annotation-22`, `independent-direct-function-call-result`, `independent-assigned-function-call-result`, `independent-aliased-function-call-result`, `independent-transitive-aliased-call-result`, `independent-generic-IO-parameter`, `independent-generic-IO-return-call`, `independent-alias-retains-callable-after-original-rebound`, `explicit-forward-string-annotation-later-import`.

Representative exact source, `independent-generic-IO-parameter` (`case.py`):

```python
import typing
def f(fp:typing.IO[str]):
    for line in fp.readlines():
        pass
```

- Intended: FURB129 case.py:56–70. Pinned Ruff observed: none.
- Basis: The resolved typing.IO[str] parameter declares the standard generic IO interface. Primary evidence: [exact-contract-cpython310.json](../../profiles/FURB129/evidence/exact-contract-cpython310.json).
- Reproduce: use the [profile](../../profiles/FURB129/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Function object versus its call result

Cases (2): `binding-8`, `independent-function-object-is-not-result`.

Representative exact source, `binding-8` (`case.py`):

```python
import typing
def fp()->typing.TextIO: pass
for line in fp.readlines():
    pass
```

- Intended: none. Pinned Ruff observed: FURB129 case.py:56–70.
- Basis: The exact source binds fp to a function object, not its return value. Primary evidence: [exact-contract-cpython310.json](../../profiles/FURB129/evidence/exact-contract-cpython310.json).
- Reproduce: use the [profile](../../profiles/FURB129/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Uncertain, eager, or branch-dependent resolution

Cases (3): `cached-namespace-1`, `uncertain-io-known-last-is-not-a-type-proof`, `eager-annotation-cannot-acquire-later-import`.

Representative exact source, `cached-namespace-1` (`case.py`):

```python
if cond:
    fp=unknown
else:
    fp=open("x")
for line in fp.readlines():
    pass
```

- Intended: none. Pinned Ruff observed: FURB129 case.py:59–73.
- Basis: CPython 3.10 exact-source branch controls produce either a standard file or a divergent reader whose readlines and iteration return different values. Primary evidence: [exact-contract-cpython310.json](../../profiles/FURB129/evidence/exact-contract-cpython310.json), [cpython310-io-contract.json](../../profiles/FURB129/evidence/cpython310-io-contract.json).
- Reproduce: use the [profile](../../profiles/FURB129/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

## Context-result regression controls

The shared helper distinguishes a context manager from its enter result. Existing resolved standard IOBase initialization remains available for synchronous with because that protocol returns the stream itself. Unknown custom manager results remain uninferred; standard IOBase has no asynchronous enter protocol. Valid different-return managers and async StringIO negative controls are retained in cases.toml alongside every original source. Current targeted evidence includes the complete superseding IO replays and actual CPython3.10/3.13 controls. Fresh full413 proof remains required before publication.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 34 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The raw reference captures retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The migration record binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
