# adapter 0.3.2

An adapter may keep nested parts and groups of cases, as a concept does. A nested part is placed by its roles: `{*part}/mod.rs` by the new spec-local `part` template, and `{*part}/adapter.rs`, `types.rs`, `errors.rs` and `constants.rs` by their own templates, read by the same role rules. The part is still the adapter: it sits beneath `adapters/`, where clients and the outside world stay allowed. `adapter_test.rs` is a case table or the namespace of its groups (`test-group-namespace` with `case-table = "exempt"`), placed by `test-group-module@0.2.0`; a group `adapter_test/{*group}/test.rs` is `scenario-test`, held to the case-table rules when it declares `CASES`; and `adapter-contract` holds the adapter whole: its `adapter_test.rs` or one of its groups runs the port's contract. 0.3.1 is removed with this version, not kept as published: no lock names it, nothing has been released.

Edited in place: places `test-shape@0.2.0` with `reads = "every-test"`, `no-runtime-fixtures@0.2.0`, `public-fields@0.3.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing). `adapter_test.rs` is read by the `group-test-shape` entry (`reads = "case-tables"`) instead of the general one: when it declares `CASES` it holds the runner shape and the table skeleton the spec-local `adapter-test` template held.

# adapter 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
