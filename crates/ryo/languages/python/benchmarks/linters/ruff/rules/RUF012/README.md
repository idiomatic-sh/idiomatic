RUF012 source candidate for Ruff 0.16.5, Python target 3.10.

The rule inspects direct class-body assignments. It diagnoses mutable literals,
comprehensions, and known mutable constructors at the value span. Annotated
`ClassVar`, `Final`, immutable annotations, dataclass fields, recognized
copy-on-instance model bases, special class attributes, and earlier `ClassVar`
targets are excluded. A local subclass of a recognized copy-on-instance model
inherits its exemption. The `ctypes` `_fields_` exemption applies to that
attribute; other mutable class attributes still share state.

The two complete upstream files under `upstream/` are byte-for-byte copies of
the fixtures at Ruff tag `0.16.5`:

- https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/ruff/RUF012.py
- https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/ruff/RUF012_deferred.py

The diagnostic contract follows
`crates/ruff_linter/src/rules/ruff/rules/mutable_class_default.rs` at that tag,
subject to Python binding semantics for import aliases, shadows, and deferred
annotations. `cases.toml` contains both complete files and independent
class-body, nested-body, import-alias, dataclass, ctypes, future-annotation,
and constructor-shadow controls. Expected source byte spans were extracted
from the pinned Ruff 0.16.5 executable.

Validation: 8/8 candidate and pinned-reference cases passed on sealed worker
`7a386a3c3b26`; 16 complete raw-exact records on current canonical helper bytes, no skipped or error rows.
`proof.json` binds the query, cases, profile, worker, and raw receipt bytes.
Canonical descriptor staging and combined admission remain pending.
