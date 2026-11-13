# PLE0643 source candidate

For a literal list or tuple with known written cardinality, this query reports
the real index-expression span when a direct integer index is at least its
length, or a direct unary-minus integer has magnitude greater than its length.
The boundary `-length` is valid for a nonempty sequence. `-0` remains zero, so an
empty sequence indexed by `-0` reports. Decoded arbitrary integers are compared
exactly; no machine-width overflow is used as a substitute for arithmetic.
Booleans, floats, unary plus, compound expressions, slices and nonliteral
receivers are outside this rule's index/receiver classification.

A direct starred element makes the receiver's cardinality unresolved for this
query. The rule declines that receiver rather than count a starred expression
as one element. A starred expression nested inside an ordinary element does not
change the outer literal's cardinality. This is an explicit independent safety
policy, not a claim that arbitrary iterable expansion has a known length.
No recursive constant evaluator or decoded-string character count is invented.

The source-pinned reference counts syntactic elements without a starred guard.
For example, with `items = [10, 20]`, `[*items][1]` accesses a valid element.
Our intended output is empty. A possible reference discrepancy is only a source
prediction until the canonical reference run and independent proof are captured;
no reference exception or admission is supplied here.

All cardinality work is keyed by the sequence owner and file. There are no
all-pair token scans. The singleton Python 3.10 profile uses the pinned
accepted-syntax phase and the existing exact suppression closure. Source cases
include the tagged upstream fixture intact and independent signed, empty,
arbitrary-precision, starred, Unicode/CRLF and suppression controls.

Policy source:
[`potential_index_error.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/pylint/rules/potential_index_error.rs),
Ruff 0.16.5 commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`.
The candidate remains unadmitted pending canonical candidate/reference evidence.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PLE0643/benchmark.toml)). **Scope:** Source-specific reference difference; upstream bug status is unverified.

The 3 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `unknown-starred`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PLE0643 --target-version py310 -- case.py
```

### Unknown starred length

Cases (1): `unknown-starred`.

Representative exact source, `unknown-starred` (`case.py`):

```python
result = [*items][1]
```

- Intended: none. Pinned Ruff observed: PLE0643 case.py:18–19.
- Basis: The length of an external starred iterable is not established by this source; CPython controls produce either IndexError or 20 from the same index, so a definite out-of-range diagnostic is unsupported. Primary evidence: [cpython310-observation.json](../../profiles/PLE0643/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/PLE0643/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Known starred expansion

Cases (2): `known-valid-starred`, `inline-literal-starred`.

Representative exact source, `inline-literal-starred` (`case.py`):

```python
result = [*[10,20]][1]
```

- Intended: none. Pinned Ruff observed: PLE0643 case.py:20–21.
- Basis: CPython expands the exact inline starred list to two elements and index 1 returns 20; the pinned reference reports an index error by counting the starred expression as one element. Primary evidence: [cpython310-observation.json](../../profiles/PLE0643/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/PLE0643/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 3 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The raw reference captures retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The migration record binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
