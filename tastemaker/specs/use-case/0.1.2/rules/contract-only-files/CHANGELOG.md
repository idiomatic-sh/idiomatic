# contract-only-files

## 0.1.0

The first version: a use case has a `service.rs`, `service_test.rs` or `fixtures/*.rs` support file only when it has a `fixtures/contract.rs`, so the language crate's SDK-facing `parse` and `project` use cases are placed without opening every use case to a service.

`constants.rs` is left out: any use case may keep its constants there, its cache bounds among them.

## 0.1.1

A Rust file directly in `fixtures/` is any use case's: its test support splits along concept into `fixtures/<concept>.rs`, which `fixtures.rs` declares. A directory below `fixtures/`, and a file of another kind, stay with the contract that owns them; the scenarios prove both stay refused.
