//! The allocation row of the shared hash index, proved on the keyed selects of the run fixtures (`fixtures::allocating`):
//! two selects keyed on one column share one index of it, so building the index and looking its keys up allocate the same
//! blocks for a hundred thousand rows as for a thousand once the worker's buffers are warm. A key copied out of every row
//! fails it; selects whose answer grows with the rows do too. It runs alone in its process, with `dhat` as the global
//! allocator, so nothing else counts.

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the one case on libtest-mimic, on this thread, so no other thread allocates beside it.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "allocation::operators::hash_index_allocations_do_not_grow_with_rows",
        || {
            assert!(
                ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::KeyedSelects {
                        answering: false
                    }
                ),
                "selects that answer no row allocate the same blocks for any number of rows once the buffers are warm"
            );
            assert!(
                !ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::KeyedSelects {
                        answering: true
                    }
                ),
                "selects whose answer grows with the rows allocate more for more rows, and the row does not hold"
            );
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}
