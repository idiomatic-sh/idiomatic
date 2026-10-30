# no-inherent-impls

## 0.1.0

The first release. It replaces the value split of the `hex` spec (value-placement and the filed `types/<value>/value.rs`): a types.rs holds plain definitions and trait impls, and a rule over a type is a service. A finding is an `impl` item with no `for` keyword in the files the entry selects.
