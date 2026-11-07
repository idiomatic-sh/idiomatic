# Imports of execution internals

Crates other than execution that import a path in execution's domain, application or outbound layers (inbound is the public use-case surface), with the concept imported and the import count.

None.

## Imports of execution's inbound layer

The public surface, for comparison.

| importing crate | execution concept | imports |
|---|---|---|
| core | execution:inbound/use_cases/execution | 127 |
| core | execution:inbound/use_cases/metadata | 85 |
| core | execution:inbound/use_cases/publication | 3 |
