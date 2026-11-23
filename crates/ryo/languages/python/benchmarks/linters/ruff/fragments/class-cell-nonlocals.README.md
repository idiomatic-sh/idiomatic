# Class cell nonlocals

`class-cell-nonlocals()` returns every `nonlocal __class__` declaration written inside a class body, at any depth of
functions: the language gives each function defined in a class an implicit `__class__` cell, so the declaration is bound
though no assignment of `__class__` encloses it. PLE0117 does not report it unbound, and F841 counts an assignment after it
as the cell's, not a local.
