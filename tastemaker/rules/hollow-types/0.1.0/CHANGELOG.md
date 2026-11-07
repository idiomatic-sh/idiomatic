# hollow-types

## 0.1.0

The `hex` spec 0.10.0's owned rule `hollow-types`, shared by the `nu-hex` and `adapter-crate` specs and changed for the plain-types model: a types.rs declares a `struct`, `enum` or `union` at its own top level, and there is no listing of filed values and no `value.rs` to check, because the value split is gone. The `hex` spec keeps its own copy until it is retired.
