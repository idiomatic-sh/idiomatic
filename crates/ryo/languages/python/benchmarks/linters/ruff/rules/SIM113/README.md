

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/SIM113/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 2 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `closure-before-initializer`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select SIM113 --target-version py310 -- case.py
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


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 2 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The [raw reference captures](../../results/discrepancy-profile-context-2026-11-05/SIM113/reference.json) retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The [migration record](../../results/discrepancy-profile-context-2026-11-05/profile-migration.json) binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
