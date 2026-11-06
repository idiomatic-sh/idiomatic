# UP008: redundant arguments to `super`

This source candidate reports explicit `super` arguments when the first argument
resolves to the enclosing class and the second resolves to the callable's first
positional parameter. It covers direct class names, one- and two-hop nested
class paths, explicit `__class__`, and qualified `builtins.super` calls whose
method has a usable class cell. It declines calls with class-name shadowing,
in-file later class rebinding, in-file nested class attribute mutation,
comprehension or intervening class scopes, and known slotted dataclass
replacement. A diagnostic spans the call's argument wrapper. Keyword
arguments may receive a diagnostic; no automatic edit is claimed.

The profile contains 35 complete cases and 47 intended findings, including
all 32 spans in the entire pinned Ruff 0.16.5 fixture. Sealed a47 native replay
matched all 35 candidate cases and all 47 intended spans. Raw Ruff output
matched 31 of 35 cases: it omitted a positional-only first parameter and
reported three class rebinding cases that CPython 3.10 demonstrates unsafe to
rewrite. The four exact raw differences have source-bound reference
discrepancy declarations; the canonical certificate validator accepted all
four. The annotated profile passed all 35 candidate and reference cases on sealed
a47 with the four certificate validations recorded. Combined admission remains
pending integration.

Static source facts cannot exclude mutation by external code or dynamic
`setattr`. The diagnostic contract does not assert universal autofix
equivalence.

Upstream fixture: `crates/ruff_linter/resources/test/fixtures/pyupgrade/UP008.py`
at Ruff 0.16.5, SHA-256
`e33031fbc11af59d578f358e4f5425bc54e607c76823c2ccba8fc8b0c20b3486`.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/UP008/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 4 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `positional-only-self`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select UP008 --target-version py310 -- case.py
```

### Positional-only self

Cases (1): `positional-only-self`.

Representative exact source, `positional-only-self` (`case.py`):

```python
class C:
    def f(self, /):
        return super(C, self).f()
```

- Intended: UP008 case.py:49–58. Pinned Ruff observed: none.
- Basis: CPython 3.10 accepts `self` as the first positional-only parameter and paired explicit and zero-argument super calls return the same inherited value. Primary evidence: [cpython310-observation.json](../../profiles/UP008/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/UP008/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Class-name rebinding after definition

Cases (3): `module-class-rebound-after-definition`, `nested-class-attribute-rebound-after-definition`, `nested-parent-attribute-rebound-after-definition`.

Representative exact source, `module-class-rebound-after-definition` (`case.py`):

```python
class C:
    def f(self):
        return super(C, self).f()
original = C
class Other: pass
C = Other
```

- Intended: none. Pinned Ruff observed: UP008 case.py:46–55.
- Basis: The exact source rebinds the module class name after preserving the original class; CPython 3.10 raises TypeError from the original method's explicit super call. Primary evidence: [cpython310-observation.json](../../profiles/UP008/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/UP008/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 4 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The [raw reference captures](../../results/discrepancy-profile-context-2026-11-05/UP008/reference.json) retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The [migration record](../../results/discrepancy-profile-context-2026-11-05/profile-migration.json) binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
