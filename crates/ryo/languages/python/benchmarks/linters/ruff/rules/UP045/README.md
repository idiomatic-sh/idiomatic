

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/UP045/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `independent-callable-lexical-ownership-0`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select UP045 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `independent-callable-lexical-ownership-0`.

Representative exact source, `independent-callable-lexical-ownership-0` (`case.py`):

```python
from typing import Optional as Opt
def f():
    value = Opt[int]
    Opt = object
```

- Intended: none. Pinned Ruff observed: UP045 case.py:56–64.
- Basis: Python compiler-local ownership makes this earlier read unbound; it does not load the outer typing import or builtin. Primary evidence: [lexical-ownership-cpython310.json](../../profiles/UP045/evidence/lexical-ownership-cpython310.json), [lexical-ownership-cpython313.json](../../profiles/UP045/evidence/lexical-ownership-cpython313.json).
- Reproduce: use the [profile](../../profiles/UP045/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 1 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The raw reference captures retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The migration record binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
