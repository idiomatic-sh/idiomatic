# format-names-in-adapters 0.1.0

The first release.

A string literal with a version suffix (`-v1`, `-v2`) written outside an adapter and its
factory is a finding: a partition's or a file's versioned name is the adapter's. The format
library itself (`postcard`, `arrow_array`) is not matched here: `client-at-adapter` reports it
outside an adapter, a factory and a composition, from the product's `libraries`.
