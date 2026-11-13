# SIM118: membership and iteration over `.keys()`

This candidate reports a no-argument `.keys()` call used as the sole right side
of `in` or as the iterable of a `for` statement or comprehension. It excludes
`self.keys()` because a method may define custom membership behavior. The rule
reports the full comparison or `for` target-through-iterator span. For a
parenthesized `for` iterator, the native syntax children identify closing `)`
tokens after the call and before the header colon, so the span includes the
outer parentheses. No claim is made that removing `.keys()` is universally
behavior-preserving for unknown receiver types.

The 20 complete cases include the entire pinned Ruff 0.16.5 fixture with its
20 expected spans. The sealed Python worker `d68b1853…` and pinned Ruff
executable each matched all 20 cases. This source remains unadmitted until
integration reviews the patch and its profile.
