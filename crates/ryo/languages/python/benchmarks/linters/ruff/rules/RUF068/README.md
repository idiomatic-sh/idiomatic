# RUF068: duplicate entry in `__all__`

Source: Ruff 0.16.5 tag `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`, [`RUF068.py`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/ruff/RUF068.py), [`duplicate_entry_in_dunder_all.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/ruff/rules/duplicate_entry_in_dunder_all.rs), and the matching pinned snapshot. The full fixture is the first case; its twelve diagnostic spans come from the snapshot. The fixture source SHA256 is `e636e2a985a47cf9b80107e49b8fc1066396c1577ee984e78da9a012e689a38a`.

The query checks each direct module-level assignment, annotated assignment, additive assignment, and one-argument `__all__.extend` call. It compares the decoded keys of string literal elements within each direct list or tuple and reports each later occurrence. It ignores dynamic elements while still checking the surrounding literals. Separate mutations are intentionally separate collections. Class and function locals, aliases, non-additive assignments, and unsupported call shapes do not count as direct `__all__` updates. The cases include those boundaries, escaped and concatenated strings, and ordinary suppression.

This is a source-only candidate. It requires an executed current-helper replay and independent comparison before admission.
