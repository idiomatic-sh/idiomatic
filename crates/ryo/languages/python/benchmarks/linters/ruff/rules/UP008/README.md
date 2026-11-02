# UP008: redundant arguments to `super`

This source candidate reports explicit `super` arguments when the first argument
resolves to the enclosing class and the second resolves to the callable's first
positional parameter. It covers direct class names, one- and two-hop nested
class paths, explicit `__class__`, and qualified `builtins.super` calls whose
method has a usable class cell. It declines calls with class-name shadowing,
in-file later class rebinding, in-file nested class attribute mutation,
comprehension or intervening class scopes, and known slotted dataclass
replacement. A diagnostic spans the call's argument wrapper. Keyword
arguments may receive a diagnostic; no automatic edit is claimed.

The profile contains 35 complete cases and 47 intended findings, including
all 32 spans in the entire pinned Ruff 0.16.5 fixture. Sealed a47 native replay
matched all 35 candidate cases and all 47 intended spans. Raw Ruff output
matched 31 of 35 cases: it omitted a positional-only first parameter and
reported three class rebinding cases that CPython 3.10 demonstrates unsafe to
rewrite. The four exact raw differences have source-bound reference
discrepancy declarations; the canonical certificate validator accepted all
four. The annotated profile passed all 35 candidate and reference cases on sealed
a47 with the four certificate validations recorded. Combined admission remains
pending integration.

Static source facts cannot exclude mutation by external code or dynamic
`setattr`. The diagnostic contract does not assert universal autofix
equivalence.

Upstream fixture: `crates/ruff_linter/resources/test/fixtures/pyupgrade/UP008.py`
at Ruff 0.16.5, SHA-256
`e33031fbc11af59d578f358e4f5425bc54e607c76823c2ccba8fc8b0c20b3486`.
