# idiomatic-rust 0.1.7

Places `error-variants-reached@0.2.0`, which counts a placed `test.rs` like a `*_test.rs`, where 0.1.5 named the earlier one. `storage-shapes-in-schemas` leaves the schemas of an adapter's documents (`outbound/<port>/adapters/<adapter>/<document>/schemas.rs`) out, as it leaves the adapter's own `schemas.rs`: a document is where one stored kind's wire shape is written, so its string table is written there. Everything else is as in 0.1.5.
