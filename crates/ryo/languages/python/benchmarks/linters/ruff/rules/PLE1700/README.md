

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLE1700/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `authored-class-base-outer-async`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLE1700 --target-version py310 -- case.py
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


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 1 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The [raw reference captures](../../results/discrepancy-profile-context-2026-11-05/PLE1700/reference.json) retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The [migration record](../../results/discrepancy-profile-context-2026-11-05/profile-migration.json) binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
