UP036 checks direct `if` and `elif` tests comparing a recognized Python version
form with a literal version. It uses existing qualified-name, semantic-child,
numeric-literal, and language-target facts. Alias resolution and shadowing stay
with the language's ordinary qualified-name relation.

The minimum-version policy handles the full `sys.version_info`, its standard
`[:2]`/`[:3]` prefixes, and major-version `.major`/`[0]` forms. Tuple components
must be literal integers; one to three components are supported. Strict and
inclusive comparisons retain their different equal-minor/micro-zero boundaries.
Invalid uint8 components report the version-specifier span; redundant tests
report the entire comparison span. Chained, reversed, boolean-combined, and
non-branch comparisons do not match this rule's policy.

Arbitrary subscripts are not equated with the major version. In particular,
`version_info[2] == 3` can take either branch within supported patch releases
with one major/minor minimum. Its exact-source discrepancy preserves pinned
Ruff's original finding separately from the intended empty expectation. The
certificate evidence explicitly models patch versions rather than claiming
those are the host interpreter's actual versions. Standard major/version forms
retain independent positive controls.

All six full upstream fixtures are retained byte-for-byte. There are 38 cases
at each of Python 3.7, 3.10 (canonical), and 3.13, including version boundaries,
aliases, shadowing, uint8 limits, normalized numeric spelling, stubs,
suppression, UTF-8 offsets, and separate physical files. All 114 pinned-reference
expectations and independent AST policy expectations were checked. Three cases
have the narrowly documented reference discrepancy above; no blanket exclusion
is used.

All 114 intended candidate/reference case pairs passed on sealed worker
`e0092188f1af6786f4981680a7129445f33117050343388ba9cb69016891266a`:
111 raw exact and three explicit micro-version discrepancies, one per target.
The [target-version supplement](../../results/full-inventory-current/up035-up036-target-supplement/README.md)
retains the initial grammar and tuple-position failures unchanged and binds the
corrected query to every successful execution. Tuple components are ranked
within `elts`; `semantic-child.ordinal` is a physical sibling position that
also counts punctuation and other fields, not a tuple-element index.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/UP036/benchmark.toml)). **Scope:** Version tuple micro-index semantics versus pinned reference finding; upstream bug status unverified.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/up035-up036-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `micro-varies`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select UP036 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (1): `micro-varies`.

Representative exact source, `micro-varies` (`case.py`):

```python
import sys
if sys.version_info[2] == 3:
    branch = "micro-three"
else:
    branch = "other-micro"
```

- Intended: none. Pinned Ruff observed: UP036 case.py:14–38.
- Basis: Index 2 of sys.version_info denotes the micro version, not the major version. Primary evidence: [micro-version-probe.py](../../profiles/UP036/evidence/micro-version-probe.py), [micro-version-inputs.json](../../profiles/UP036/evidence/micro-version-inputs.json).
- Reproduce: use the [profile](../../profiles/UP036/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/up035-up036-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
