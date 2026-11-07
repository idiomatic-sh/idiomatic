# RUF033 dataclass post-init defaults

This source candidate reports each default expression on a direct `__post_init__`
method in a standard-library dataclass. It uses the existing decorator-origin
fragment, the function argument wrappers, and source locations. It does not
claim a fix. The profile targets Python 3.12 because the complete Ruff 0.16.5
fixture includes a PEP 695 method type parameter.

Fourteen complete controls include the entire pinned fixture and cover alias
imports, decorator calls, positional-only and keyword-only defaults, nested
classes, non-dataclasses, asynchronous methods, shadowed decorators, and
`# noqa`. The 34 rule spans in `cases.toml` combine CPython AST offsets with observed
pinned Ruff output. The full fixture also expects one shared `invalid-syntax`
finding for a duplicate parameter name. Sealed a47 native and pinned Ruff replays each passed all 14 cases with
35 complete findings. Combined admission remains pending integration.
