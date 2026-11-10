# cargo-workspace

## 0.13.0

`pub_use` is no longer denied: re-export policy is owned by Tastemaker's `idiomatic.arch/no-reexports` rule, which allows the
one-item seam forwards and the port-factory resource-handle export and refuses a re-export outside an SDK surface, so clippy
holds no second, blunter definition of it. Every scenario is unchanged apart from the line.
