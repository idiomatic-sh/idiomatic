# ordered-output 0.1.0

The first version. A `HashMap`, `HashSet` (std, `hashbrown`, `FxHashMap`, `FxHashSet`, `AHashMap`, `AHashSet`) or
`DashMap` named inside a declaration that derives `Serialize` is a finding: what it writes comes out in an order that
changes from run to run. The serialized type holds a `BTreeMap`, a `BTreeSet` or a sorted `Vec` instead.

## Fixtures

`report-with-hash-map` (failing), `ordered-report-and-lookup-index` (passing).
