# application-service 0.1.2

Places the files of a nested part by their roles, where 0.1.1 placed only the operation's own top-level files and a product listed each nested file in its manifest. A nested part is a directory at any depth beneath the operation (`{*part}`): its module file `mod.rs`, which holds its items, its `service.rs`, `types.rs`, `constants.rs`, `errors.rs`, `interface.rs`, `schemas.rs` and `fixtures.rs`, its test support beneath `fixtures/`, its `test.rs` and `service_test.rs`, and its groups of cases `service_test/{*group}/` (`mod.rs`, `test.rs`, `fixtures.rs`); the operation's own test support beneath `fixtures/` and its own groups of cases are placed the same way. Every new entry has `count = { min = 0 }`. A new spec-local template, `part`, places the module, errors, interface, schemas and proof files of a nested part with no content of its own to require. Scenarios `nested-part-is-placed` (nothing closed) and `file-of-no-role-in-a-nested-part` (closed). Everything else is as in 0.1.1.

# application-service 0.1.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.1.0 named the earlier ones. Nothing else changes.
