# ASYNC116 source candidate

This query reports the real call span for a proven imported `trio.sleep` or
`anyio.sleep`, with exactly one argument whose expression is an integer or float
literal greater than 86400. The named argument is `seconds` for Trio and `delay`
for AnyIO. Calls need not be awaited or occur inside an asynchronous function.

The optional numeric relation supplies parser-decoded values: radices,
underscores, arbitrary integer precision, and overflow-to-infinity float literals
do not require source-spelling approximations. Strict greater uses `numeric_gte`
in both directions and an antijoin; `86400.25` is retained. Booleans, complex
numbers, unary signs, computed expressions, names and `math.inf` are separate
syntax and are not constant-folded. Unknown call origins are not inferred from
attribute spelling. Argument counting includes starred and keyword expansion
nodes, but only an actual direct literal expression can satisfy the value test.

All joins use existing owner/file keys. There are no token-pair scans or shared
helper changes. The singleton profile declares Python 3.10, the canonical
accepted-syntax phase and the exact existing suppression-module closure.

Policy audit uses Ruff 0.16.5, commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`:
[`long_sleep_not_forever.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/flake8_async/rules/long_sleep_not_forever.rs).
The tagged source fixture is included intact with published snapshot spans
transcribed against its source; additional controls have independent expected
outputs. Source authoring and static metadata checks are not runtime
parity evidence; canonical candidate/reference validation remains pending.
