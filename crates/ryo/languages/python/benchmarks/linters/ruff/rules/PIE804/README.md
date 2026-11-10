Implements Ruff 0.16.5 `unnecessary_dict_kwargs`. Only keyword expansions belonging to call expressions qualify; class header arguments do not. Each dictionary key must be an ordinary string literal and a valid non-keyword Python identifier. FString and TString expressions are excluded even when they contain only text.

Dictionary key/value cardinalities distinguish ordinary dictionaries, mixed expansions, and the special single nested expansion. Nested call or dictionary expansions inside a value do not change the outer dictionary's structure. Duplicate keyword names affect Ruff's fix safety, not whether it reports a diagnostic.

All original 13 cases remain. Added controls include the exact tagged fixture and preserved full-report combined-run reproductions.

Source pin: `astral-sh/ruff` tag `0.16.5`, Python target `py310`. New source controls use actual pinned reference output; final cross-rule inputs reuse sealed reference records without rerunning Ruff.
