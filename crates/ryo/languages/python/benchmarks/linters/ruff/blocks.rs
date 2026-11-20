//! Spike (ryo-v1np): engine bake-off, "blocks". A standalone path: the existing front end compiles each rule to its `Plan`
//! (tapped out of `compile_rules`), then per file: parse, one provider walk filling typed u64 columns for the relations the
//! loaded rules read, lazily built per-file indexes (dense by node position, or hashed), and per-rule programs lowered from
//! each rule's own plan, run per candidate with bound keys pushed down (sideways binding). Nothing of the file pass, merged
//! program, operators run service or store runs.
#![allow(
    dead_code,
    missing_docs,
    unused,
    clippy::all,
    reason = "throwaway bake-off prototype, ryo-v1np"
)]
mod adapter;
mod composition;
mod constants;
mod frontend_projection;
mod schemas;
mod settings;
#[path = "blocks_engine.rs"]
mod engine;

fn main() -> std::process::ExitCode {
    match engine::main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
