# Scope lookup of reads

`scope-lookup-uses(reads=<reference, file>, uses=<reference, binding, file>)` returns the binding each given read resolves to under Ruff 0.16.5's `resolve_load`: the latest earlier binding of the name in the nearest scope that has one, whatever branch it sits in. The provider's lexical `uses` stand where its binding is in that nearest scope or nearer; a nearer binding replaces them, and a read the provider left unbound gets the binding it finds.

Scopes are searched outward from the read's own scope. Class scopes other than the read's own are skipped. Binds in a comprehension's enclosing scopes count only when they precede the read, except across a function or lambda boundary, whose body runs after its enclosing scope and sees that scope's final bindings. A `global` or `nonlocal` declaration moves its binds to the function that declares them. Deletions do not hide a binding.

Callers pass the reads that are not deferred forward references: those resolve through `deferred-read-events`.
