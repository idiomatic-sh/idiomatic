//! The allocation profile of a run, taken under `dhat`: what the `blocks` test bounds and the `lint` bench's `--dhat` run
//! measures. The process that calls it has `dhat::Alloc` as its global allocator and runs nothing beside it.

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
