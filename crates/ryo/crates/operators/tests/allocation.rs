//! The allocation rows of the operators, proved on the workloads of the run fixtures (`fixtures::allocating`): the shared
//! hash index allocates the same blocks for a hundred thousand rows as for a thousand once the worker's buffers are warm; a
//! closure fused into its join allocates the same blocks over ten thousand chains as over a hundred, walking only the
//! sources its left rows bind; and once warm, the graph kernels allocate nothing over a file's edges. Past the bound, each
//! workload breaks its row: selects whose answer grows with the rows, the join over the whole closure, and the kernels cold.
//! It runs alone in its process, with `dhat` as the global allocator, so nothing else counts.

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the one case on libtest-mimic, on this thread, so no other thread allocates beside it.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "allocation::operators::allocations_do_not_grow_with_rows_or_the_graph",
        || {
            assert!(
                ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::Workloads {
                        past_bound: false
                    }
                ),
                "the workloads within the bound allocate the same blocks for any number of rows or unbound chains, and the warm graph kernels none"
            );
            assert!(
                !ryo_operators::inbound::use_cases::run::fixtures::allocating::held(
                    &ryo_operators::inbound::use_cases::run::fixtures::allocating::Workloads {
                        past_bound: true
                    }
                ),
                "each workload past the bound allocates more for more rows or chains, or allocates cold, and the row does not hold"
            );
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}
