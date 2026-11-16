//! The allocation row of the whole per-file path: `lint` over the core's own corpus, the fake language and every
//! operator's rule, and the most blocks a file allocates for it beyond what the parser does on its own, which for the fake
//! is nothing. No language's corpus is read here: a language's own allocation test is `tests/allocation.rs` of the language.
//! The measuring is the core's contract, the same for every product. It runs alone in its process, with `dhat` as the global
//! allocator, so nothing else counts. Run as a test it holds the rows (the rules give their rows, streaming and output);
//! run with `--bmf` it prints the measure as one Bencher Metric Format document on stdout, which asymptote holds to its best.

use ryo_core::inbound::use_cases::lint::fixtures::{corpus, measures};

/// The allocator that counts every block, so the row measures what the path allocates.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// What the parser does for a file by itself: the fake language reads a file without allocating.
fn library(_bytes: &[u8]) {}

/// Runs the one case on libtest-mimic, which answers libtest's command line and reports as cargo's harness does; a red
/// exits 101, as a failed cargo test does. The case runs on this thread, so no other thread allocates beside it.
#[expect(
    clippy::print_stdout,
    reason = "with `--bmf` the document on stdout is what the asymptote tool reads"
)]
fn main() {
    if std::env::args().any(|each| each == "--bmf") {
        let beyond = measures::allocation(&corpus::fake_corpus(), &library);
        println!("{{\"fake\":{{\"blocks-per-file-beyond-parser\":{{\"value\":{beyond}}}}}}}");
    } else {
        let mut arguments = libtest_mimic::Arguments::from_args();
        arguments.test_threads = Some(1);
        let case = libtest_mimic::Trial::test(
            "allocation::core::reading_allocates_as_the_parser_library_does",
            || {
                let _beyond = measures::allocation(&corpus::fake_corpus(), &library);
                Ok(())
            },
        );
        libtest_mimic::run(&arguments, vec![case]).exit();
    }
}
