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
