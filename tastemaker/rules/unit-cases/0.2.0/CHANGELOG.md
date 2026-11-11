# unit-cases 0.2.0

A unit test that is the namespace of its groups (a `service_test.rs` that only declares `service_test/<group>/`, the form `test-group-namespace` and the `service-test` template accept) is read with the tables of its groups: the rows of every file beneath `<part>_test/` count as its rows. 0.1.0 read only the file itself, so a namespace, whose cases are its groups, was reported as having no `succeeds` or `refuses` row. Scenarios `namespace-whose-groups-hold-the-rows` (pass) and `namespace-whose-groups-refuse-nothing` (reported). Everything else is as in 0.1.0, which stays as published: tastemaker and asymptote lock it.

Edited in place: scenario `namespace-read-through-the-language-placement`. The rule reads the facts of the files its entry includes and no others, so an entry that places it on a part includes the part's group tests too (`*_test/**/test.rs`); with only `src/domain/*/*.rs` a correct namespace is reported (red).

Edited in place: a `*_test.rs` beneath a `*_test/` directory is read for the rows of the namespace above it and is no unit test of its own (scenario `group-file-is-read-as-rows-not-as-a-unit-test`); entries read `*_test/**/*.rs`.
