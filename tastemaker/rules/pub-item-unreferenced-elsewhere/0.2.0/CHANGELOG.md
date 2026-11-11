# pub-item-unreferenced-elsewhere 0.2.0

Two roles are not read. A field of a type in a `schemas.rs` is the wire shape serde reads and writes by name, kept public so the schema is plain data; and an item in a `fixtures.rs` or under `fixtures/` is test support, whose fake's fields stay public for the tests that build it. 0.1.0 reported both, and the only way past was an ignore per item. Scenarios `unreferenced-service-function` (reported), `schema-field-named-by-no-other-file` and `fixture-function-named-by-no-other-file` (not reported). Everything else is as in 0.1.0.
