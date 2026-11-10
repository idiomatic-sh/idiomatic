PLW2101 reports a fresh threading Lock, RLock, Condition, Semaphore, or BoundedSemaphore constructed directly as a with-context. It inspects every context item, uses unique imported binding origins, and preserves the full call span. Shared lock variables, unrelated factories, shadowing, deletion, reassignment, suppression, Unicode, and multiline calls have controls.

The production query passes all34 intended full-report cases on sealed worker755bda55. Exact query/module/profile/case hashes and raw reports are in evidence. Native Ruff0.16.5 differs on unrelated-call-before-lock: it returns before checking later items. The intended diagnostic remains; CPython3.10.19 executes the exact source with an instrumented real Lock factory and confirms later construction. Raw oracle output is unchanged. Formal reference-discrepancy admission remains pending, so this profile is not READY.

Source audit: tagged0.16.5 crates/ruff_linter/src/rules/pylint/rules/useless_with_lock.rs, plus statement dispatch. Independent implementation uses relational call/import/context matching rather than reproducing the visitor's early return. No shared fragments or provider code changed.

Current canonical singleton proof:34/34intended pass,33raw exact+1documented reference discrepancy; existing stage_binding/native verify-cases on sealed6540. The exact independent real-lock context evaluation evidence is pinned per case.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLW2101/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `unrelated-call-before-lock`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLW2101 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `unrelated-call-before-lock`.

Representative exact source, `unrelated-call-before-lock` (`case.py`):

```python
import threading
with resource(), threading.Lock(): pass
```

- Intended: PLW2101 case.py:34–50. Pinned Ruff observed: none.
- Basis: All with-context items evaluate from left to right. Primary evidence: [cpython310-exact.json](../../profiles/PLW2101/evidence/cpython310-exact.json), [cpython310-exact.py](../../profiles/PLW2101/evidence/cpython310-exact.py).
- Reproduce: use the [profile](../../profiles/PLW2101/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 1 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The raw reference captures retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The migration record binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
