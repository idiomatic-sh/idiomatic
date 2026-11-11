# item 0.1.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.1.0 named the earlier ones. Nothing else changes.

Edited in place: places `test-shape@0.2.0` with `reads = "every-test"`, `no-runtime-fixtures@0.2.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing).
