# hollow-types

## 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A `types.rs` is now owned by its file: `direct` is the set of files with a top-level type, the module declarations of a `types.rs` take its root from their own path, and the declared module files are kept by joining the set of files that declare `value`. The roots with no owning file are the findings. The findings and the scenarios are as in 0.1.0.

## 0.1.0

The `hex` spec 0.10.0's owned rule `hollow-types`, shared by the `nu-hex` and `adapter-crate` specs and changed for the plain-types model: a types.rs declares a `struct`, `enum` or `union` at its own top level, and there is no listing of filed values and no `value.rs` to check, because the value split is gone. The `hex` spec keeps its own copy until it is retired.
