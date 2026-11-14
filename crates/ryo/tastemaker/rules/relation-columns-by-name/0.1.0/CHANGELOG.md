# relation-columns-by-name

## 0.1.0

The first version. Placed over `crates/query_language/src/**`, `crates/derived/src/**` and `crates/core/src/**` by ryo's
tastemaker.toml, tests, fixtures and acceptance steps left out: a
`ColumnIndex(<integer literal>)` call or tuple-struct pattern, an or-pattern of literals among them, outside a `const` initializer,
is reported. Derived from the planner reading a `syntax-child` edge's `field` (its column 3) as the `syntax-tree` element's `file`
(its column 3). Fixtures `edge-column-read-as-element-column`, `or-pattern-of-numbers`, `qualified-number`,
`named-columns`, `test-file-left-out`.
