# Unpacking targets

`unpacking-targets()` returns the target nodes whose names the pinned Ruff 0.16.5 counts as unpacked: a tuple or list target
of an assignment whose value is not a tuple, list or set display, or that shares its statement with a target that is not a
container (`(x, y) = coords = 1, 2`); and the tuple a `with` or `async with` item binds after `as` (a list there is not one).
The names below these roots are unused-unpacked (RUF059) rather than unused (F841): F841 leaves them out and RUF059 reports
them.
