# SIM107 return in try or except and finally

This source candidate reports the first direct `return` statement in a `finally`
body when the same `try` or one of its direct exception handler bodies contains
a direct `return`. It includes `try` and `try*`, uses the existing syntax and
child relations, and suppresses `# noqa` directives. It claims no edit.

The Python 3.12 profile covers `try*`. Eleven complete controls include the
entire pinned Ruff 0.16.5 fixture and direct return, nested return, else,
async, and suppression cases. Six expected spans match pinned Ruff in the first a47 replay. Sealed a47 native and pinned Ruff replays each passed all 11 cases and
six findings. Combined admission remains pending integration.
