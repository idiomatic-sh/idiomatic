# threads-at-adapter 0.1.0

The first v2 release: the query of its schema-1 release without the `paths` param, which the rule entry's `include` and `exclude` now give. Code on the schema-1 `arch.threads-at-adapter` moves up by moving its `paths` to the entry's `include` and its `boundaries` to the entry's `exclude`.

Rewritten for rayon: a `rayon::ThreadPoolBuilder` (and its methods), a `rayon::ThreadPool`'s methods, and rayon's global `spawn`, `spawn_fifo`, `scope`, `scope_fifo` and `in_place_scope` are pools too. The same check runs as clippy `disallowed-methods` in the `clippy-config` template; this rule stays until products run that lint.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
