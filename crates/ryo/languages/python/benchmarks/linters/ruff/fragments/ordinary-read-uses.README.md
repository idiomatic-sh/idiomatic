# Uses of ordinary reads

`ordinary-read-uses(reads=<reference, file>, uses=<reference, binding, file>)` gives the binding each given read uses. A read that `lookup-read-events` resolves (stage 2: the read's own scope, then its enclosing scopes, at the read's position) uses the binding the lookup finds; a read it does not resolve keeps the provider's use.

A function or lambda body is read after its enclosing scope, so its reads carry later positions and see that scope's final bindings without a rule of their own. The name lookup itself lives only in `lookup-read-events`: class scopes, handler-cleanup restoration, deletions and global/nonlocal homes are its rules. Callers pass the reads that are not deferred forward references, which resolve through `deferred-read-events`.
