UP036 checks direct `if` and `elif` tests comparing a recognized Python version
form with a literal version. It uses existing qualified-name, semantic-child,
numeric-literal, and language-target facts. Alias resolution and shadowing stay
with the language's ordinary qualified-name relation.

The minimum-version policy handles the full `sys.version_info`, any subscript of
it, and `.major`. As the pinned Ruff 0.16.5 does, a subscript is compared as the
whole version when its comparator is a tuple and as the major version when it is
an integer, whatever the index or slice (`[0:2]`, `[1:]`, `[2]`, `[x]`). Tuple components
must be literal integers; one to three components are supported. Strict and
inclusive comparisons retain their different equal-minor/micro-zero boundaries.
Invalid uint8 components report the version-specifier span; redundant tests
report the entire comparison span. Chained, reversed, boolean-combined, and
non-branch comparisons do not match this rule's policy.

All six full upstream fixtures are retained byte-for-byte. There are 38 cases
at each of Python 3.7, 3.10 (canonical), and 3.13, including version boundaries,
aliases, shadowing, uint8 limits, normalized numeric spelling, stubs,
suppression, UTF-8 offsets, and separate physical files. Every expectation is the
pinned Ruff's.

The target-version supplement
retains the initial grammar and tuple-position failures unchanged and binds the
corrected query to every successful execution. Tuple components are ranked
within `elts`; `semantic-child.ordinal` is a physical sibling position that
also counts punctuation and other fields, not a tuple-element index.
