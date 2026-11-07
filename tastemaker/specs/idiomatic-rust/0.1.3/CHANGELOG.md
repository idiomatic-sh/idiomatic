# idiomatic-rust 0.1.3

`constants-once` leaves out a file named `test.rs`, the case table of a grouped test, as it leaves out `*_test.rs`: the values of two case tables are not a constant declared twice. `one-type-per-name` leaves out `cli/src/outcome.rs`, whose name `Outcome` the `cli/outcome` template requires. Otherwise as 0.1.2.
