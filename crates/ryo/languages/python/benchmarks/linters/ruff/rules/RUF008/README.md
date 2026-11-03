RUF008 candidate for the pinned Ruff 0.16.5 default policy, target Python 3.10.

The query checks direct annotated assignments in recognized dataclass class bodies, with the first recognized decorator controlling the dataclass kind. It preserves the stable-mode boundary: the preview-only `field(default=...)` expansion is not enabled. Mutable literal/comprehension/constructor defaults are excluded for ClassVar or immutable annotations. Suppression uses the diagnosed default's line and range.

Source contract: `crates/ruff_linter/src/rules/ruff/rules/mutable_dataclass_default.rs` at tag `0.16.5`, its dataclass helpers, `ruff_python_stdlib::typing` immutable catalogs, and `checkers/ast/mod.rs::visit_expr` annotation deferral. Complete copies of all three tagged fixtures are retained under `upstream/`.

The annotation-origin helper handles the finite module families consumed by the immutable/ClassVar policies, not arbitrary qualified-name construction. Immediate known bindings stay cached. Unresolved immediate names use the final module namespace; deferred native and embedded annotation names use the final lexical namespace. Import aliases, collection-module chains, builtin fallback and shadows remain explicit. Lookup is restricted to requested annotation descendants. The shared lookup-symbol-events fragment is copied unchanged from the integration tree.

Validation: 100/100 full-report cases against both the pinned Ruff executable and frozen actual benchmark worker `097f5c…`; 200 complete records, no errors, skipped rows or Unknown rule/file pairs. This includes the three tagged fixtures, stdlib/attrs decorators, mutable and immutable shapes, suppression, 38 paired deferred/cached-name, import-alias, class-shadow, stub and builtin controls. `proof.json` binds the tested query and helper bytes. This is bounded fixture evidence; default registration and cross-rule cohort validation remain integration work.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/RUF008/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `late-annotation-cache-0`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select RUF008 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `late-annotation-cache-0`.

Representative exact source, `late-annotation-cache-0` (`case.py`):

```python
from dataclasses import dataclass
@dataclass
class C:
    x: ClassVar[list[int]] = []
from typing import ClassVar
```

- Intended: RUF008 case.py:83–85. Pinned Ruff observed: none.
- Basis: An eager unresolved ClassVar annotation cannot acquire the later typing import. Primary evidence: [cpython310.json](../../profiles/RUF008/evidence/cpython310.json), [cpython313.json](../../profiles/RUF008/evidence/cpython313.json).
- Reproduce: use the [profile](../../profiles/RUF008/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
