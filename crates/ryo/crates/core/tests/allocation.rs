//! The allocation row of the whole per-file path: `lint` over the core's own corpus, the fake language and every
//! operator's rule, allocates no more for a file than the parser does on its own, which for the fake is nothing. No
//! language's corpus is read here: a language's own allocation test is `tests/allocation.rs` of the language. The
//! measuring and the comparison are the core's contract, the same for every product. It runs alone in its process,
//! with `dhat` as the global allocator, so nothing else counts.

use ryo_core::inbound::use_cases::lint::fixtures::{corpus, measures};

/// The allocator that counts every block, so the row measures what the path allocates.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// What the parser does for a file by itself: the fake language reads a file without allocating.
fn library(_bytes: &[u8]) {}

/// The case's stable id, which its line, its failure and a validation that owns its red name exactly.
const CASE: &str = "per_file_path::allocates_as_the_parser_library_does";

/// Runs the case and reports it as cargo's test harness does (`test <id> ... ok|FAILED` and a `test result:` summary), so
/// what reads a test log reads this one; a red exits 101, as a failed cargo test does.
#[expect(
    clippy::print_stdout,
    reason = "a harness = false runner's report is its standard output, in the shape of cargo's harness"
)]
fn main() -> std::process::ExitCode {
    println!("\nrunning 1 test");
    let passed = std::panic::catch_unwind(|| {
        measures::allocation(&corpus::fake_corpus(), &library);
    })
    .is_ok();
    if passed {
        println!(
            // ignore: idiomatic.arch/no-silent-skip - cargo's own summary line, which run.py parses: it reports 0 tests ignored, no skip
            "test {CASE} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
        );
        return std::process::ExitCode::SUCCESS;
    }
    println!(
        "test {CASE} ... FAILED\n\nfailures:\n    {CASE}\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n"
    );
    std::process::ExitCode::from(101)
}
