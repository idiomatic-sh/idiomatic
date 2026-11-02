RUF013 source candidate for Ruff 0.16.5, Python target 3.10.

The rule inspects parameters whose direct default is `None`, resolves their
annotations through lexical import and builtin facts, and propagates explicit
nullability only through `Optional`, `Union`, `Literal`, `Annotated`, PEP 604
unions, and parsed quoted annotations. The first type argument of a top-level
`Annotated` determines the diagnostic span. Unknown local aliases are held
conservatively because they may already include `None`. Failed quoted parses
inside unions are excluded.

The five complete upstream files under `upstream/` are byte-for-byte copies of
the fixtures at Ruff tag `0.16.5`:

- https://github.com/astral-sh/ruff/tree/0.16.5/crates/ruff_linter/resources/test/fixtures/ruff
  (`RUF013_0.py` through `RUF013_4.py`)

The diagnostic contract follows
`crates/ruff_linter/src/rules/ruff/rules/implicit_optional.rs` and
`crates/ruff_linter/src/rules/ruff/typing.rs` at that tag. `cases.toml`
contains all five complete files and independent controls for qualified
imports, aliases, unions, future annotations, positional-only and keyword-only
parameters, and unresolved local aliases. Expected source byte spans were
extracted from the pinned Ruff 0.16.5 executable.

Validation: 12/12 candidate and pinned-reference cases passed on sealed worker
`7a386a3c3b26`; 24 complete raw-exact records on current canonical helper bytes, no skipped or error rows.
`proof.json` binds the query, cases, profile, worker, and raw receipt bytes.
Canonical descriptor staging and combined admission remain pending.
