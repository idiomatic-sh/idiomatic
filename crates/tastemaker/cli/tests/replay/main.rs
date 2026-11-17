//! The replay suite: the tests that run Tastemaker over a real tree, one module each; `scaffold` writes a fresh language and holds it whole, and holds `tm scaffold` over each sample, its rendered parse contract held by `contract`; `samples` holds each sample under `crates/tastemaker/examples/` to its documented findings and fixes; `languages` is what the cases of `init`, `create`, `validate`, `check` and `explain` run each command over each sample with.

mod benchmark_fixture;
mod check;
#[path = "../fixtures/replay/contract.rs"]
mod contract;
mod create;
mod explain;
#[path = "../fixtures/replay/fixes.rs"]
mod fixes;
mod imports;
mod init;
mod languages;
#[path = "../fixtures/replay/locks.rs"]
mod locks;
#[path = "../fixtures/replay/pinned.rs"]
mod pinned;
mod samples;
mod scaffold;
mod validate;
