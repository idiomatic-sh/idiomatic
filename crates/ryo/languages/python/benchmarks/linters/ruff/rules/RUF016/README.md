Implements Ruff 0.16.5 `invalid_index_type`. A slice is a valid index shape; its lower, upper, and step expressions are checked individually. A known invalid bound reports its own real expression span. Integer, boolean, and None bounds are accepted; unknown expressions are left unchecked, matching `CheckableExprType`.

The existing 18 cases remain unchanged. Added controls retain the final RUF015 fixture interaction, valid and invalid slices, unknown unary expressions, and the exact tagged RUF016 fixture. Full combined reference arrays are retained beside the scoped profile.

Source pin: `astral-sh/ruff` tag `0.16.5`, Python target `py310`. New source controls use actual pinned reference output; final cross-rule inputs reuse sealed reference records without rerunning Ruff.
