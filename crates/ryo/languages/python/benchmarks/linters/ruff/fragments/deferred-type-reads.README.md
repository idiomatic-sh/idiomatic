# Deferred and unvisited type operands

`deferred-type-reads(expressions=<id: node, file: file>)` returns a visit category for supplied read occurrences: `deferred` for roots postponed by the pinned checker and `unvisited` for operands the checker skips. Other reads have no row.

Native annotations are postponed as whole expression roots when their scoped future flag is enabled, including runtime-valued call arguments below those roots. Stub type-definition expression roots, stub annotations, stub class headers, quoted type payloads, native type-parameter bounds/defaults, and PEP 695 alias values retain their respective checker deferral rules. Native PEP 613 values and runtime TypeVar bound calls are not postponed solely because a future import exists.

The projection reuses the exact type-definition-context catalogue and flag transitions. A skipped operand never becomes a concrete declaration. F821 uses the result to decide whether to replay or suppress a frontend read; usage consumers resolve concrete deferred declarations at their actual later checkpoint.
