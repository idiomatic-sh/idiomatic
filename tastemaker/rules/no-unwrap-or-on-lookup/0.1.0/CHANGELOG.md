# no-unwrap-or-on-lookup 0.1.0

The first release. In production code, a `.get(..)` by a typed index's number (`place.0`), optionally through `copied` or
`cloned`, answered by `unwrap_or` or `unwrap_or_default` is reported: a lookup in an internal table by its own index fails
loudly when it misses. Lookups by a key and slices cut by a range are not. The `idiomatic-rust` spec places it on every
product's production code.
