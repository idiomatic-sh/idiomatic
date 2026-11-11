# use-case 0.1.3

A group of the use case's cases may hold groups of its own: the group module and its test are placed at any depth beneath `use_case_test/` (`{*group}`, where 0.1.2 placed one level, `{group}`), and a group's test support `use_case_test/{*group}/fixtures.rs` is placed with `count = { min = 0 }`. Scenario `nested-use-case-test-group`. Everything else is as in 0.1.2. 0.1.2 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility.

# use-case 0.1.2

A use case places its test support along concept as `fixtures/<concept>.rs`, which `fixtures.rs` declares, with no contract: `contract-only-files` refuses only a directory below `fixtures/` and a file of another kind. The rules that excuse `fixtures.rs` excuse these files too.

# use-case 0.1.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.1.0 named the earlier ones. Nothing else changes.
