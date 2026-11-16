# value-module

## 0.2.0

The content names no visibility for `value`: `mod value;` is met by `pub mod value`, `pub(crate) mod value` and a private
`mod value` alike, so a value module under a module that is not public declares `pub(crate) mod value` and rustc's
`unreachable_pub` holds it, where 0.1.0 demanded `pub mod value` there and the two could not both pass (the fused
integration, ryo-2964.8). Scenario `crate-visible-value` is new. Everything else is as in 0.1.0.

## 0.1.0

The `hex` spec 0.10.0's owned template `value-module`, shared by the `nu-hex` and `adapter-crate` specs. Its content is unchanged; the `hex` spec keeps its own copy until it is retired.
