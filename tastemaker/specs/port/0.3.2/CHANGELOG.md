# port 0.3.2

Places its adapters at `adapter@0.3.2`, which keeps nested parts and groups of cases. 0.3.1 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility. Everything else is as in 0.3.1.

Edited in place: places `no-runtime-fixtures@0.2.0`, `public-fields@0.3.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

Edited in place: the `unit-cases` entry reads `*_test/**/*.rs` beside its own files, the flat groups and the group directories of a namespace test, which the rule reads only when its entry includes them.

# port 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
