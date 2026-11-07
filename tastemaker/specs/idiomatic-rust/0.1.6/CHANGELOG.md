# idiomatic-rust 0.1.6

`storage-shapes-in-schemas` leaves the schemas of an adapter's documents (`outbound/<port>/adapters/<adapter>/<document>/schemas.rs`) out, as it leaves the adapter's own `schemas.rs`: a document is where one stored kind's wire shape is written, so its string table is written there. Everything else is as in 0.1.5.
