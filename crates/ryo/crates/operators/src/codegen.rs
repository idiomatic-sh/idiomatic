//! Spike (ryo-or7l, engine bake-off): the rule plans as generated native code. `gen` prints the merged logical program's
//! per-rule plans as Rust (one push function per plan node, monomorphised and inlined into nested loops); `rt` is the minimal
//! per-file host the printed code runs on (fact columns read in place, per-file indexes built once and shared by every rule,
//! literal codes resolved once per file); `generated` is the printed pack, compiled ahead of time into the bench worker.
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    reason = "Spike (ryo-or7l): throwaway bake-off prototype; plain indexing is the compiled loop measured"
)]

pub mod generated;
pub mod generator;
pub mod rt;
