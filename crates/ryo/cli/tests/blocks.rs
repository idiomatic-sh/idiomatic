//! The big blocks of a run: what is big in a worker is owned by the worker and kept between files, and what is big in a rule that
//! reads across files is the tables of its one plan, decoded once for the rule from every file's buckets, so the calls that allocate a
//! block of a megabyte or more are bounded by the workers and the rules, each with its tables, never by the files. Counted from the
//! allocation profile `dhat` writes, as the sites whose blocks average a megabyte or more; it runs alone in its process, over a tree
//! it writes. The same bound over all of ruff is the `lint` bench's `--dhat` run (asymptote's `memory-dhat`).

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot run its rules has nothing to assert: the panic is the failure of the test"
)]

use std::fmt::Write as _;

#[path = "fixtures/replay/base.rs"]
mod base;
#[path = "fixtures/replay/counting.rs"]
mod counting;
#[path = "fixtures/replay/profile.rs"]
mod profile;
#[path = "fixtures/replay/unbounded.rs"]
mod unbounded;

/// The allocator that records every allocation of the run into the profile the blocks are counted from.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// Runs the one case on libtest-mimic, which answers libtest's command line (`--list`, `--exact`, a filter) as cargo's harness does,
/// so listing the tests runs no lint; a red exits 101, as a failed cargo test does. The case runs on this thread, so the profile is
/// of the run alone.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "blocks::the_big_blocks_of_a_run_are_bounded_by_the_workers_and_the_rules",
        || {
            big_blocks();
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}

/// How many files the tree holds, and how many functions of how many statements each: more files than a worker owns tables.
const FILES: usize = 200;
/// The functions of each file.
const FUNCTIONS: usize = 5;
/// The statements of each function.
const STATEMENTS: usize = 20;

/// A tree of [`FILES`] Rust files, each of [`FUNCTIONS`] functions, the same functions in every file so the rules that read across
/// files find them, each statement commented.
fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for file in 0..FILES {
        let mut text = String::new();
        for function in 0..FUNCTIONS {
            writeln!(text, "pub fn work_{function}() {{").expect("writing to a String cannot fail");
            for statement in 0..STATEMENTS {
                writeln!(
                    text,
                    "    let value_{statement} = {function} + {statement}; // TODO {statement}"
                )
                .expect("writing to a String cannot fail");
            }
            text.push_str("}\n");
        }
        let directory = root.path().join(format!("file_{file}"));
        std::fs::create_dir_all(&directory).expect("a directory");
        std::fs::write(directory.join("lib.rs"), text).expect("a file");
    }
    root
}

/// Lints a tree it writes under the allocation profile and bounds the calls that allocated a block of a megabyte or more.
fn big_blocks() {
    let tree = tree();
    // The run keeps its store as a lint does, in a cache directory of its own.
    let cache = tempfile::tempdir().expect("a cache directory");
    let rules: Vec<_> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    // Every rule the run asks for may read across files: each is at most one crossing.
    let crossing = rules.len();
    let (counted, profile) = profile::profiled(|| {
        unbounded::lint_counting(tree.path(), rules, base::store_at(cache.path()))
            .expect("the rules run")
    });
    // The workers the engine runs on: as many as the machine has, since the run asks for none.
    let workers = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    profile::bounded(&counted, &profile, workers, crossing, "the tree");
}
