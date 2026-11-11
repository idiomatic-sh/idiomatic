# derived-view-acquisition

## 0.2.1

A build call through the semantic port is accepted in a fixtures role as well as in a proof-placed file: a `fixtures.rs` or a file beneath `fixtures/` whose module, or a module above it, is declared under the test or fixtures gate, `#[cfg(test)]` or `#[cfg(any(test, feature = "fixtures"))]` exactly, as `no-runtime-fixtures` reads it; a `cfg` that names the feature without gating on it (`not(feature = "fixtures")`, or an `any` whose other arm holds in a production build) is no gate (scenarios `negated-fixtures-gate-port-build` and `always-true-fixtures-gate-port-build`, refused), so a recording fake of the port needs no per-file proof placement. A production build call is still refused, and so is the same call in a fixtures file declared without the gate. Scenarios `domain-port-build-is-production` and `ungated-fixtures-port-build` (refused), `gated-fixtures-port-build` (pass). Everything else is as in 0.2.0; 0.3.0 does not carry this change. Module files are named for their directory (`<x>.rs`, never `<x>/mod.rs`), so every module-file path the query and the scenarios name is `<x>.rs`.

## 0.2.0

Who reaches the derived owner is confined: an identifier of the new required `confined` param (the owner's acquiring entries and the leased views and handles it issues), written in a production file outside the owner tree, is a finding unless the new required `allowed` param lists that `{ file, name }`. It replaces the derived census's text scans of the acquiring entries' callers (`sites`) and of the files that hold a leased view or handle (`Holder::ALL`). Fixtures `listed-holder-and-owner-may-name-the-view` (pass), `unlisted-caller-of-an-acquiring-entry` and `unlisted-holder-of-a-leased-view` (violations). Everything else is as in 0.1.0.

## 0.1.0

Edited in place: the adapters, fake and contract of the derived port are its implementations and build through it; a file outside `outbound/derived` that names the port and calls `build` is still a consumer and is refused.
