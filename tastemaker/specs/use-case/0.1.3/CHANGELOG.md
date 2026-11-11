# use-case 0.1.3

A group of the use case's cases may hold groups of its own: the group module and its test are placed at any depth beneath `use_case_test/` (`{*group}`, where 0.1.2 placed one level, `{group}`), and a group's test support `use_case_test/{*group}/fixtures.rs` is placed with `count = { min = 0 }`. A group module is `test-group-module@0.2.0`, held by `test-group-namespace@0.1.0`: it declares modules only, and a group that only gathers nested groups is a namespace. A group test, `use_case_test/{*group}/test.rs`, is a `unit-test` with its own case table and runners, as in 0.1.2, read by `test-shape` at any depth. The optional `service_test.rs` is a case table or the namespace of its groups, placed by `test-group-module@0.2.0` and held by `test-group-namespace` with `case-table = "exempt"`, which holds a case table to the skeleton the `unit-test` template writes. Scenarios `nested-use-case-test-group` (placed) and `nested-group-test-plain-scenario` (refused). Places `no-runtime-fixtures@0.2.0` and `public-fields@0.3.0`, where 0.1.2 named their earlier versions, so a product that places it locks one version of each rule. 0.1.2 is removed with this version, not kept as published: no lock names it, nothing has been released.

# use-case 0.1.2

A use case places its test support along concept as `fixtures/<concept>.rs`, which `fixtures.rs` declares, with no contract: `contract-only-files` refuses only a directory below `fixtures/` and a file of another kind. The rules that excuse `fixtures.rs` excuse these files too.

# use-case 0.1.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.1.0 named the earlier ones. Nothing else changes.
