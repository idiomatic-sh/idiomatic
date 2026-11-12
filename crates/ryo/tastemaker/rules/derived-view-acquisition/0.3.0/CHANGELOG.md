# derived-view-acquisition

## 0.3.0

Who reaches the derived owner is decided by where the code sits, not by a list: the `allowed` param (a `{ file, name }` per caller and holder) is removed, and the new required `owner-concept` param names the path of the owner's concept directory beneath a crate's `src/` (`application/file_dag/derived`). A confined identifier written in a production file beneath that directory is the owner's own; a file beneath another directory that is only named `derived` is not; anywhere else it is a finding, so acquisition is collapsed into one owner module per crate. The 0.2.0 exclusion of `application/file_dag/derived/owner/**` is that directory. Scenarios `owner-concept-may-name-the-view` (pass), `acquisition-outside-the-owner-concept` and `holder-outside-the-owner-concept` (violations) replace the three of 0.2.0 that read `allowed`, and `another-directory-named-derived` (violation) holds the concept to its one path. A build call through the semantic port is accepted in a fixtures role as well as in a proof-placed file: a `fixtures.rs` or a file beneath `fixtures/` whose module, or a module above it, is declared under a `cfg` naming `test` or the `fixtures` feature, so a recording fake of the port needs no per-file proof placement; a production build call is still refused, and so is the same call in a fixtures file declared without the gate. Scenarios `domain-port-build-is-production` and `ungated-fixtures-port-build` (refused), `gated-fixtures-port-build` (pass): the change of 0.2.1, carried here so that the two lines of work meet in one version. Everything else is as in 0.2.0.

Edited in place: scenario inputs name module files `<dir>.rs`, never `<dir>/mod.rs` (owner, 2026-11-11).

Edited in place: the fixtures gate is exact, as in 0.2.1: a fixtures module counts as gated only under `#[cfg(test)]` or `#[cfg(any(test, feature = "fixtures"))]`; a `cfg` that names the feature to negate it, or an `any` whose other arm holds in a production build, is no gate. Scenarios `negated-fixtures-gate-port-build` and `always-true-fixtures-gate-port-build` (refused) are carried from 0.2.1.

Edited in place: the derived port's own outbound files, beneath `outbound/derived` (its interface, factory, adapters and fakes), are the port, not its consumers, so a confined identifier such as `DerivedPort` written there is not a finding; a production file anywhere else that names the port still is. Scenarios `port-hex-may-name-the-port` (pass) and `non-port-file-naming-the-port` (violation).

Edited in place: that exemption is the port's own name only: an identifier the port's `outbound/derived/interface.rs` defines, written beneath the same crate's `outbound/derived`. Any other confined identifier there (a held `AcquiredView`) is a finding, and so is the port's name beneath another crate's `outbound/derived`. Scenarios `port-hex-file-holding-a-view` and `another-crate-outbound-derived-naming-the-port` (violations).

## 0.2.0

Who reaches the derived owner is confined: an identifier of the new required `confined` param (the owner's acquiring entries and the leased views and handles it issues), written in a production file outside the owner tree, is a finding unless the new required `allowed` param lists that `{ file, name }`. It replaces the derived census's text scans of the acquiring entries' callers (`sites`) and of the files that hold a leased view or handle (`Holder::ALL`). Fixtures `listed-holder-and-owner-may-name-the-view` (pass), `unlisted-caller-of-an-acquiring-entry` and `unlisted-holder-of-a-leased-view` (violations). Everything else is as in 0.1.0.

## 0.1.0

Edited in place: the adapters, fake and contract of the derived port are its implementations and build through it; a file outside `outbound/derived` that names the port and calls `build` is still a consumer and is refused.
