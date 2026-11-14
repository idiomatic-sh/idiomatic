//! The allocation row of the kernels, proved on the programs of the run fixtures (`fixtures::allocating`): two selects keyed
//! on one column share one index of it, and a union, an intersection and a projection drop repeated rows through
//! `hash_index`'s keyed rows, so each allocates the same blocks for a hundred thousand rows as for a thousand once the
//! worker's buffers are warm. A key copied out of every row fails it; programs whose answer grows with the rows do too. It
//! runs alone in its process, with `dhat` as the global allocator, so nothing else counts.

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the one case on libtest-mimic, on this thread, so no other thread allocates beside it.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "allocation::operators::kernels_allocate_nothing_per_row",
        || {
            assert!(
                ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::Kernels {
                        answering: false
                    }
                ),
                "programs that answer a few rows allocate the same blocks for any number of rows once the buffers are warm"
            );
            assert!(
                !ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::Kernels {
                        answering: true
                    }
                ),
                "programs whose answer grows with the rows allocate more for more rows, and the row does not hold"
            );
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}
