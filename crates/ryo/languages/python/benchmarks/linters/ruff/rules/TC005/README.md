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
