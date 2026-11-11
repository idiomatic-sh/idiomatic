# sdk 0.1.3

Places its entries at `entry@0.1.2`, which places `no-runtime-fixtures@0.2.0`. 0.1.2 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility. Everything else is as in 0.1.2.

Edited in place: places `no-runtime-fixtures@0.2.0`, where it named its earlier version, so a product that places it locks one version of the rule: two versions of one rule share its id, and the earlier entries ran the later query. Its own `no-reexports@0.4.0` entry is removed: `idiomatic-rust` places `no-reexports@0.6.0` over the same files (every file but the SDK root, its resource modules, their contracts and the macros they call), so a product that places both holds one version of the rule; scenario `reexport-outside-resource`, which read that entry, goes with it.

# sdk 0.1.2

Places the `entry` spec at 0.1.1, whose entries may split their test data into a `fixtures/` directory; nothing else changes.
