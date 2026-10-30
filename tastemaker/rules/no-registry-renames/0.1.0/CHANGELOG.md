# no-registry-renames

## 0.1.0

The first release. `core-dependencies` reads a core's inherited dependency (`foo.workspace = true`) by its key, which is its package only while the workspace table does not rename a registry crate; this rule makes that hold. The devtool starter's shared rule block places it on the product's root `Cargo.toml`.

A finding is an entry of `[workspace.dependencies]` with a `package` key different from its own key and no `path`. Not findings: an entry with a `path` (a crate of the product, renamed as ryo's `ryo-*` aliases are), a `package` equal to the key, and any rename in a member's own `[dependencies]` (`core-dependencies` reads those by package).
