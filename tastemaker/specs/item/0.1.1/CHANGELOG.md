# item 0.1.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.1.0 named the earlier ones. Nothing else changes.

Edited in place: places `no-runtime-fixtures@0.2.0`, where it named its earlier version, so a product that places it locks one version of the rule: two versions of one rule share its id, and the earlier entries ran the later query.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

Edited in place: the `unit-cases` entry reads `*_test/**/*.rs` beside its own files, the flat groups and the group directories of a namespace test, which the rule reads only when its entry includes them.
