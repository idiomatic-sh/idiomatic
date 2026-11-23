# Logger objects

`logger-objects()` returns every attribute node `X.m` whose object `X` is a logger as the pinned Ruff 0.16.5 decides it: `X` resolves to the `logging` module or to `flask.current_app.logger`; or `X` is a call of `logging.getLogger` or `logging.Logger`; or `X` is a name, or an attribute whose last name, matches `log*`, `*logger`, `*logging`, `LOG*`, `*LOGGER` or `*LOGGING` and does not resolve to an import or a builtin. G010 (`X.warn`) and LOG014 (`X.info(..., exc_info=True)`) ask it which method calls are logger calls; the method name is the caller's.
