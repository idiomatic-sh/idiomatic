# no-filter-map-parse 0.1.0

The first release. It reports every `filter_map` whose arguments call `parse` or `from_str`: a value that does not parse
would be dropped as if it were never there. The `idiomatic-rust` spec places it on every Rust file but test data.
