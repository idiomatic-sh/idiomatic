# BLE001 candidate: broad exception handlers

This candidate is source-authored and unpromoted. No native candidate/reference
execution or parity certificate is attached. The isolated profile selects BLE001
under the pinned Ruff 0.16.5 shipped settings, Python 3.10 parser configuration,
and `accepted-syntax` applicability. Existing canonical helper bytes are reused;
no parser, engine, relation reader or shared fragment is changed.

The rule reports the first builtin `Exception` or `BaseException` reached by
recursing through the handler type's tuple elements, with that member's exact
span. Arbitrary descendants of calls, subscriptions and attributes are not tuple
members. Existing independent builtin-read matching supplies language origins;
local names that merely resemble builtin exception names do not become builtins.

The two exemptions traverse statements in the handler body, including control
flow bodies. A nested try, function or class is a boundary. A bare raise, direct
raise of the handler name, or use of that name as a direct cause exempts the
handler. Only a call that is itself an expression statement can supply a logging
exemption; calls inside assignments, conditions, arguments, awaits or boolean
expressions do not qualify.

Logging origin and policy are deliberately separate. Imported `logging`,
`flask.current_app.logger`, imported logging methods and inline
`logging.getLogger`/`logging.Logger` constructors have explicit source origins.
An imported foreign object does not become logging because its name ends in
`logger`. The selected rule also has an explicit **compatibility heuristic** for
unqualified local or unresolved dotted names whose final normalized spelling
starts with `log`/`LOG` or ends with `logger`, `logging`, `LOGGER` or `LOGGING`.
That spelling is not proof of the object's type or runtime behavior. The query
keeps it in `heuristic_receivers` and combines it with proven origins only at the
rule-policy boundary; no language fact or provider provenance is rewritten.
Fixtures named `local-spelling-policy-*` document this deliberate policy, including
rebinding to an arbitrary value, while imported-foreign and arbitrary-receiver
controls remain diagnostics. Custom `logger-objects` settings are outside this
shipped-default profile.

An `exception` method exempts unconditionally. Other recognized methods require
an explicit `exc_info` keyword whose expression is not classified definitely
false by the selected policy. Parser-decoded primitive properties handle numeric
zero, booleans, null, and decoded empty strings/bytes. Empty containers, empty
f-strings and recursive bare builtin iterable constructors are also classified.
F-string dependencies preserve conversion, format-specification and debug
boundaries. Calls on imported/shadowed constructors remain unknown. Boolean and
unary expressions remain unknown; this is the selected rule's conservative
classifier, not a claim that their runtime truthiness is unknowable. There is no
recursive BoolOp capability gap and no finite-depth approximation.

The 119 full-report fixtures contain 118 independently authored controls and the
exact 293-line upstream fixture. Their 83 intended BLE001 spans include 26 spans
transcribed from the checked-in upstream snapshot. All fixture Python sources
parse as Python 3.10 under a static `ast.parse` check, and the TOML/profile/module
closure has been read as source. These checks do not execute RyoQL or Ruff. The
next required step is canonical strict profile staging followed by native
candidate/reference execution when the integration measurement quiet period ends.

Source audit: Ruff tag `0.16.5`, commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`:

- [Rule and both exemption visitors](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/flake8_blind_except/rules/blind_except.rs).
- [Logger origin and explicit spelling policy](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_semantic/src/analyze/logging.rs).
- [Logger method vocabulary](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/flake8_logging/helpers.rs).
- [Statement visitor boundaries](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_ast/src/statement_visitor.rs).
- [Truthiness and empty f-string classification](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_ast/src/helpers.rs).
- [Builtin binding distinction](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_semantic/src/model.rs).
- [Upstream fixture](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/flake8_blind_except/BLE.py), SHA256 `6fdf98763dca9a207a366b521146276d73f55a6d0560a438ba7ad57b34ce2073`.
- [Checked-in snapshot](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/flake8_blind_except/snapshots/ruff_linter__rules__flake8_blind_except__tests__blind-except_BLE.py.snap), SHA256 `44dcd2528e2296ed9fa45c1ae44b84e07aacd9c5d3171265be02068b3be0609b`.
