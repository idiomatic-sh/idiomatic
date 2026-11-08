# Imports of execution internals

Crates other than execution that import a path in execution's domain, application or outbound layers (inbound is the public use-case surface), with the concept imported and the import count.

| importing crate | imports | execution concepts imported |
|---|---|---|
| core | 37 | execution:outbound/digest (12); execution:outbound/facts (13); execution:outbound/matching (3); execution:outbound/operators (2); execution:outbound/query_language (7) |

## Imports of execution's inbound layer

The public surface, for comparison.

| importing crate | execution concept | imports |
|---|---|---|
| core | execution:inbound/use_cases/execution | 102 |
| core | execution:inbound/use_cases/metadata | 99 |
| core | execution:inbound/use_cases/prepare | 18 |
| core | execution:inbound/use_cases/publication | 4 |
