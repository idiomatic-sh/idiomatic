# idiomatic-sh-ryo-evaluation

A spike, never published and never merged: fused, lazy evaluation of the compiled plans `execution::compile_rules` answers. Every
rule's root `select ... from "syntax-tree" where kind = ...` is merged into one kind index and served by one walk over the parser's
tree; every other relation is built the first time a rule that still has candidates asks for it, and kept until the next file. Facts
reach the evaluator through one port, `outbound::facts`, whose adapters (`syntax`, `location`) are the only code that knows how a
relation is made.
