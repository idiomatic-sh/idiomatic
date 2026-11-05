# RUF010 explicit string conversion in an f-string

This candidate reports a call to the built-in `str`, `repr`, or `ascii` when it
is the whole expression of an f-string field without a conversion flag or debug
`=` marker. It accepts one ordinary positional argument; `str(object=...)` is
also accepted. It uses qualified origins and suppresses `# noqa` directives.
The diagnostic span is the conversion call. No automatic edit is claimed.

The profile targets Python 3.12 because the complete pinned Ruff 0.16.5 fixture
contains comments inside f-string expressions. Python 3.10 rejects that syntax
at line 69 and produces 11 extra `invalid-syntax` findings in both tools; 3.12
accepts the full fixture. The 10 complete cases contain 37 intended RUF010
findings, including 31 in the full pinned fixture. A sealed Python worker
(`8b09a477…`) and pinned Ruff executable each matched all 10 cases under the
3.12 profile. Source remains unadmitted until integration reviews the handoff.
