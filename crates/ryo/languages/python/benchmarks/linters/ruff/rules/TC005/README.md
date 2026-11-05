# TC005 source candidate

This rule reports the real `pass` span when an `if` guard denotes a type-checking
block, its body contains exactly that statement, and it has no `elif` or `else`.
The singleton profile targets Python 3.10 and uses the pinned accepted-syntax
phase. It is pending canonical execution, not admitted coverage.

Imported aliases use the existing binding-aware modernization module. An alias
shadowed by a local declaration is not assumed to retain its import origin.
The bare spelling `TYPE_CHECKING` and an attribute named `TYPE_CHECKING` are also
accepted as an explicit rule compatibility convention. These spellings do not
establish a Python semantic fact about the object's value or origin. Other bare
names and compound conditions are not broadened into this policy. An inner
qualifying `if` is checked independently; an outer block containing an inner `if`
does not have a single-pass body.

The independently authored cases cover import aliases, shadowing, compatibility
spellings, statement counts, clauses, nesting, stub files, comments, suppression,
CRLF, and Unicode byte offsets. The tagged upstream fixture is included intact
with independently specified full expected spans.

Policy audit: Ruff tag `0.16.5`, commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`:

- [`empty_type_checking_block.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/flake8_type_checking/rules/empty_type_checking_block.rs)
- [`is_type_checking_block`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_semantic/src/analyze/typing.rs)

The query uses keyed edges and body counts; it introduces no token-pair joins,
shared helper changes, or engine changes.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/TC005/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 2 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `alias-late-local-assignment`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select TC005 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (2): `alias-late-local-assignment`, `independent-callable-lexical-ownership-0`.

Representative exact source, `alias-late-local-assignment` (`case.py`):

```python
from typing import TYPE_CHECKING as TC
def f():
    if TC:
        pass
    TC = False
```

- Intended: none. Pinned Ruff observed: TC005 case.py:67–71.
- Basis: Python compiler-local ownership makes this earlier read unbound; it does not load the outer typing import or builtin. Primary evidence: [lexical-ownership-cpython310.json](../../profiles/TC005/evidence/lexical-ownership-cpython310.json), [lexical-ownership-cpython313.json](../../profiles/TC005/evidence/lexical-ownership-cpython313.json).
- Reproduce: use the [profile](../../profiles/TC005/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
