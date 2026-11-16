//! The big blocks of a run over ruff: what is big in a worker is owned by the worker and kept between files, and what is big in
//! a rule that reads across files is the tables of its one plan, decoded once for the rule from every file's buckets, so the calls
//! that allocate a block of a megabyte or more are bounded by the workers and the rules, each with its tables, never by the files. Counted from the
//! allocation profile `dhat` writes, as the sites whose blocks average a megabyte or more; it runs alone in its process.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot run its rules has nothing to assert: the panic is the failure of the test"
)]

#[path = "fixtures/replay/base.rs"]
mod base;
#[path = "fixtures/replay/counting.rs"]
mod counting;

/// The allocator that records every allocation of the run into the profile the blocks are counted from.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// How many tables a worker, or a rule that reads across files, owns at most: the steps and inputs of the widest plan, and its scratch.
const TABLES: usize = 64;

/// A megabyte.
const BIG: u64 = 1_048_576;

/// Runs the one case on libtest-mimic, which answers libtest's command line (`--list`, `--exact`, a filter) as cargo's harness does,
/// so listing the tests runs no lint; a red exits 101, as a failed cargo test does. The case runs on this thread, so the profile is
/// of the run alone.
fn main() {
    let mut arguments = libtest_mimic::Arguments::from_args();
    arguments.test_threads = Some(1);
    let case = libtest_mimic::Trial::test(
        "blocks::the_big_blocks_of_a_run_over_ruff_are_bounded_by_the_workers_and_the_rules",
        || {
            big_blocks();
            Ok(())
        },
    );
    libtest_mimic::run(&arguments, vec![case]).exit();
}

/// Lints ruff under the allocation profile and counts the calls that allocated a block of a megabyte or more.
fn big_blocks() {
    let profile = std::env::temp_dir().join(format!("ryo-blocks-{}.json", std::process::id()));
    // The run keeps its store as a lint does, in a cache directory of its own.
    let cache = tempfile::tempdir().expect("a cache directory");
    let rules: Vec<_> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    // Every rule the run asks for may read across files: each is at most one crossing.
    let crossing = rules.len();
    let counted = {
        let _profiler = dhat::Profiler::builder().file_name(&profile).build();
        counting::lint_counting(&base::snapshot::ruff(), rules, base::store_at(cache.path()))
            .expect("the rules run")
    };
    // The run read the files and gave findings: a run that read nothing allocates no big block for them.
    assert!(
        counted.reads > 0 && !counted.rows.is_empty(),
        "the run read {} files, the project {} times, and gave {} findings",
        counted.reads,
        counted.projects,
        counted.rows.len()
    );
    assert!(
        counted.unread.iter().all(|path| {
            !std::path::Path::new(path.as_str())
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        }),
        "a Rust file of ruff was left unread"
    );
    let text = std::fs::read_to_string(&profile).expect("the profile");
    // The profile is a temporary file: one left behind, when it cannot be removed, changes no count.
    drop(std::fs::remove_file(&profile));
    let sites: serde_json::Value = serde_json::from_str(&text).expect("a profile");
    // A profile with no allocation site, or a site without its bytes and blocks, measured nothing: it is refused, never counted
    // as no big block.
    let profiled = sites
        .get("pps")
        .and_then(serde_json::Value::as_array)
        .filter(|profiled| !profiled.is_empty())
        .expect("the profile holds the run's allocation sites");
    let big: u64 = profiled
        .iter()
        .map(|site| {
            let (bytes, blocks) = (
                site.get("tb").and_then(serde_json::Value::as_u64),
                site.get("tbk").and_then(serde_json::Value::as_u64),
            );
            let (bytes, blocks) = bytes.zip(blocks).unwrap_or_else(|| {
                panic!("an allocation site without its bytes and blocks: {site}")
            });
            u64::from(blocks > 0 && bytes / blocks.max(1) >= BIG).saturating_mul(blocks)
        })
        .sum();
    // The workers the engine runs on: as many as the machine has, since the run asks for none.
    let workers = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let most = u64::try_from((workers + crossing) * TABLES).unwrap_or(u64::MAX);
    assert!(
        big <= most,
        "{big} blocks of a megabyte or more were allocated, more than {workers} workers and {crossing} rules with {TABLES} tables each: something is allocated again for the files"
    );
}
