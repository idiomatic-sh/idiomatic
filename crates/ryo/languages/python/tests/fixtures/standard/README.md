# python standards

The standards the language's kinds are named by, pinned. The node kinds are `CPython 3.15.0rc2 ast`, the token kinds
`CPython 3.15.0rc2 token`; the extras are what the tree has that CPython 3.15.0rc2 ast and token lack.

## Source

- `Python.asdl`: CPython's `Parser/Python.asdl` (github.com/python/cpython, tag `v3.15.0rc2`), copied unchanged: every constructor
  of it is a node kind (`nodes/service_test.rs`).
- `<relation>.ast`: what CPython 3.15.0rc2's own `ast.dump(ast.parse(source))` prints for `relations/<relation>/input.py`, recorded
  once with that interpreter (`uv python install 3.15.0rc2`) and never edited. The language's tree of the same input, projected
  onto the standard (tokens and extras dropped), must have the same nodes (`adapter_test.rs` of the adapter). The `syntax_error`
  input is not Python, so CPython parses no tree of it and it has none.
- `corpus/<file>.ast` and `suite/valid/<path>.ast`: the same, recorded with the same interpreter, for every file of `corpus/` and of
  `suite/valid/`, at the same path. Every one must agree but one, the check's one stated exception:
  `suite/valid/inline/unary_plus_py315.py`, where CPython folds the key `+1` of a mapping pattern into a `Constant` and the library
  keeps the `UnaryOp` written.
- `refused/<case>.py` and `refused/<case>.refused`: a file CPython's parser refuses and the library reads without an error, and
  what that CPython said, `line:column: message`. The language reports the same message on the same line (`adapter_test.rs`).
  `else_without_except` is ruff's `suite/valid/statement/try.py` in one case: a `try` with `else` and no `except`;
  `unknown_future` imports a feature `__future__` does not have.
- `future_features.txt`: `__future__.all_feature_names` of CPython 3.15.0rc2, one a line, printed by that interpreter: the
  features the adapter's `FUTURE_FEATURES` must list, in order (`adapter_test.rs`).

## Version

CPython `3.15.0rc2` (`3.15.0rc2 (main, Sep  1 2026, 14:16:41) [Clang 22.1.3]`, as `sys.version` gives it), the Python the library reads (`PythonVersion::latest()` of `ruff_python_ast` 0.0.18). Its licence is
`NOTICE-cpython-LICENSE`, beside this page.

- `builtin_names.txt`: the public names (plus `__build_class__`, `__debug__`, and `__import__`) of
  `dir(builtins)` from the pinned CPython 3.15.0rc2 interpreter with `-I`, including default `site`
  additions. The qualification service compares its entire inventory to this independent fixture.
  Introduction versions follow the Python Library Reference's version annotations; they are
  numeric facts, not the caller's target version. In particular, `aiter`/`anext` require 3.10,
  `BaseExceptionGroup`/`ExceptionGroup` require 3.11, and `PythonFinalizationError` requires 3.13.

  Version annotations: https://docs.python.org/3.15/library/functions.html and
  https://docs.python.org/3.15/library/exceptions.html. The shared builtin inventory test
  and explicit 3.9/3.10 and 3.10/3.11 cases prevent parser-version availability from being
  confused with a consumer target. Default-site names follow https://docs.python.org/3.15/library/site.html.
