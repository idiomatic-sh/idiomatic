//! The allocation row of the engine, proved on the language double of the parse fixtures (`fixtures::allocating`): the
//! allocations of a language's own library parse (`read_tree`) are in the count of reading and in the count of the baseline
//! alike, so they cancel, and only the table and the readers are judged. A `read_tree` that allocates a scratch buffer in
//! every file passes; a reader that allocates in every element fails. It runs alone in its process, with `dhat` as the
//! global allocator, so nothing else counts.

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the one case on libtest-mimic, on this thread, so no other thread allocates beside it.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "allocation::language::reading_allocates_as_the_parser_library_does",
        || {
            assert!(
                ryo_language::inbound::use_cases::parse::fixtures::allocating::held(
                    &ryo_language::inbound::use_cases::parse::fixtures::allocating::AllocatingLanguage {
                        greedy_reader: false
                    }
                ),
                "a library parse that allocates in every file is the library's, and the row holds"
            );
            assert!(
                !ryo_language::inbound::use_cases::parse::fixtures::allocating::held(
                    &ryo_language::inbound::use_cases::parse::fixtures::allocating::AllocatingLanguage {
                        greedy_reader: true
                    }
                ),
                "a reader that allocates in every element is not the library's, and the row does not hold"
            );
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}
