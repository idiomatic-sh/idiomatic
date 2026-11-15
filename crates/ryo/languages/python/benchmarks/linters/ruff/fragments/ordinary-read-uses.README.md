# Uses of ordinary reads

`ordinary-read-uses(reads=<reference, file>, uses=<reference, binding, file>)` gives the binding each given read uses. A read that `lookup-read-events` resolves (stage 2: the read's own scope at its position, enclosing scopes at their final state across a function or lambda boundary) uses the binding the lookup finds; a read it does not resolve keeps the provider's use.

The name lookup itself lives only in `lookup-read-events`: class scopes, handler-cleanup restoration, deletions and global/nonlocal homes are its rules. Callers pass the reads that are not deferred forward references, which resolve through `deferred-read-events`.
