//! The allocation profile of a run, taken under `dhat`, and the bound on its big blocks: what the `blocks` test bounds over a tree it
//! writes. The process that calls it has `dhat::Alloc` as its global allocator and runs nothing beside it.

/// What a profile says of a run.
pub(crate) struct Profile {
    /// The bytes live when the run held most (`t-gmax`).
    pub gmax_bytes: u64,
    /// The blocks allocated at sites whose blocks average a megabyte or more.
    pub big_blocks: u64,
}

/// A megabyte.
const BIG: f64 = 1_048_576.0;

/// Runs `run` under a profiler and answers what it returned and what its profile says.
pub(crate) fn profiled<T>(run: impl FnOnce() -> T) -> (T, Profile) {
    let file = std::env::temp_dir().join(format!("ryo-blocks-{}.json", std::process::id()));
    let (returned, gmax_bytes) = {
        let _profiler = dhat::Profiler::builder().file_name(&file).build();
        let returned = run();
        // The bytes live when the run held most, read before the profiler ends and writes its file.
        (returned, dhat::HeapStats::get().max_bytes)
    };
    let text = std::fs::read_to_string(&file).expect("the profile");
    // The profile is a temporary file: one left behind, when it cannot be removed, changes no count.
    drop(std::fs::remove_file(&file));
    let sites: serde_json::Value = serde_json::from_str(&text).expect("a profile");
    let big_blocks: u64 = sites
        .get("pps")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|site| {
            let (bytes, blocks) = (site.get("tb")?.as_f64()?, site.get("tbk")?.as_f64()?);
            (blocks > 0.0_f64 && bytes / blocks >= BIG).then_some(site.get("tbk")?.as_u64()?)
        })
        .sum();
    (
        returned,
        Profile {
            gmax_bytes: u64::try_from(gmax_bytes).expect("bytes that fit 64 bits"),
            big_blocks,
        },
    )
}

/// How many tables a worker, or a rule that reads across files, owns at most: the steps and inputs of the widest plan, and its scratch.
pub(crate) const TABLES: usize = 64;

/// Holds a run over `target` and its profile to the bound on big blocks: what is big in a worker is owned by the worker and kept
/// between files, and what is big in a rule that reads across files is the tables of its one plan, decoded once for the rule from
/// every file's buckets, so the calls that allocate a block of a megabyte or more are bounded by the `workers` and the `crossing`
/// rules, each with its [`TABLES`], never by the files.
///
/// # Panics
///
/// When the run read nothing or gave no finding, left a Rust file unread, the profile saw nothing, or the big blocks are more than
/// the bound.
pub(crate) fn bounded(
    counted: &super::counting::Counted,
    profile: &Profile,
    workers: usize,
    crossing: usize,
    target: &str,
) {
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
        "a Rust file of {target} was left unread"
    );
    // A profile that saw no allocation counted nothing: the allocator is the profiler's.
    assert!(
        profile.gmax_bytes > 0,
        "the profile saw the run's allocations"
    );
    let big = profile.big_blocks;
    let most = u64::try_from((workers + crossing) * TABLES).unwrap_or(u64::MAX);
    assert!(
        big <= most,
        "{big} blocks of a megabyte or more were allocated, more than {workers} workers and {crossing} rules with {TABLES} tables each: something is allocated again for the files"
    );
}
