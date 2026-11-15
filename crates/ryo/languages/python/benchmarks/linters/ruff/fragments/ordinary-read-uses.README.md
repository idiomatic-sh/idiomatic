# Uses of ordinary reads

`ordinary-read-uses(reads=<reference, file>)` gives the binding each given read uses: the binding `lookup-read-events` finds for it at stage 2, the read's own scope and then its enclosing scopes at the read's position. A read that resolves to a deletion, a cleanup or nothing uses no binding.

A function or lambda body is read after its enclosing scope, so its reads carry later positions and see that scope's final bindings without a rule of their own. The name lookup itself lives only in `lookup-read-events`: class scopes, handler-cleanup restoration, deletions and global/nonlocal homes are its rules. Callers pass the reads that are not deferred forward references, which resolve through `deferred-read-events`.
