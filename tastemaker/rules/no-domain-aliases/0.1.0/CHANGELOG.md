# no-domain-aliases

## 0.1.0

The `hex` spec 0.10.0's owned rule `no-domain-aliases`, shared by the `nu-hex` and `adapter-crate` specs. The query is unchanged: it reads declarations, never the layout. Only the words that named the hex or its steps changed; the `hex` spec keeps its own copy until it is retired. An alias of a function pointer type is admitted: the rule and `clippy::type_complexity` disagreed on a long fn-pointer field, and for a function pointer the alias wins (scenario `function-pointer-alias`).
