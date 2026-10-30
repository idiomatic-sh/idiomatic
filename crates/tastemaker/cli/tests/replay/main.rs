//! The replay suite: the tests that run Tastemaker over a real tree, one module each; `scaffold` writes a fresh language and holds it whole, and holds `tm scaffold` over each sample; `samples` holds each sample under `crates/tastemaker/examples/` to its documented findings and fixes; `languages` is what the cases of `init`, `create`, `validate`, `check` and `explain` run each command over each sample with.

mod check;
mod create;
mod explain;
#[path = "../fixtures/replay/fixes.rs"]
mod fixes;
mod init;
mod languages;
mod samples;
mod scaffold;
mod validate;
