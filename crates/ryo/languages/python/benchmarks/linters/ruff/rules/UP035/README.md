UP035 follows Ruff 0.16.5's deprecated-import mapping at pinned commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`, including target-version gates.
The query uses spelled absolute `from` imports rather than read-time symbol
inference: an alias or later rebinding does not undo an import diagnostic.
Relative imports, star imports, plain module imports, and unrelated modules do
not match the mapping.

Moved members produce one diagnostic per destination module per statement.
Renamed members produce one diagnostic per imported occurrence, including
repeated members. The emission key retains this distinction, so several
findings can share the entire import statement span. Suppression is applied to
the statement, then joined back to those distinct diagnostic identities.

The canonical singleton uses Python 3.10. Counter-target profiles retain all
38 cases at each of Python 3.7, 3.8, 3.9, 3.11, 3.12, 3.13, and 3.14; they are
not reduced negative-only suites. Every target includes the complete upstream
UP035 fixture, every pinned mapping family, aliases/rebinding, nested imports,
relative/star/plain imports, stubs, repeated/grouped findings, UTF-8 offsets,
multiline suppression placement, and separate physical files with overlapping
node positions.

All 304 candidate/reference case pairs passed raw exact on sealed worker
`e0092188f1af6786f4981680a7129445f33117050343388ba9cb69016891266a`.
The [target-version supplement](../../results/full-inventory-current/up035-up036-target-supplement/README.md)
binds the final query, full cases, profiles, transitive helpers, worker, and
collector to all eight targets. The initial grammar failure is retained in its
immutable archive; it is not counted as passing evidence.
