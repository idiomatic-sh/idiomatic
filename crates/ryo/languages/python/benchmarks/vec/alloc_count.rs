//! THROWAWAY (ryo-fmx3.1 redo-alloc): the harness's global allocator, one cargo feature away from each other: glibc's
//! (none given), mimalloc (`vec-mimalloc`, as the workspace pins it), jemalloc (`vec-jemalloc`, what the command line,
//! ruff and uv ship on Linux), or dhat's counting heap profiler (`vec-dhat`: every allocation counted with its call stack;
//! slow, for counts only, never timed). The crate forbids unsafe code, so the counter is dhat's, not a wrapper of ours.

#[cfg(feature = "vec-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "vec-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "vec-dhat")]
#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

/// The allocator's name, for the run's lines.
pub(super) const NAME: &str = if cfg!(feature = "vec-mimalloc") {
    "mimalloc"
} else if cfg!(feature = "vec-jemalloc") {
    "jemalloc"
} else if cfg!(feature = "vec-dhat") {
    "dhat"
} else {
    "glibc"
};

/// The run's profiler (`vec-dhat`; its file is `VEC_DHAT_OUT`, default `dhat-heap.json`), started at the run's start.
pub(super) struct Counter {
    #[cfg(feature = "vec-dhat")]
    _p: dhat::Profiler,
}

pub(super) fn start() -> Counter {
    Counter {
        #[cfg(feature = "vec-dhat")]
        _p: dhat::Profiler::builder().file_name(std::env::var("VEC_DHAT_OUT").unwrap_or_else(|_| "dhat-heap.json".to_owned())).trim_backtraces(Some(12)).build(),
    }
}

/// (blocks allocated, bytes allocated, peak bytes live) so far; zeros without `vec-dhat`.
pub(super) fn counts() -> [u64; 3] {
    #[cfg(feature = "vec-dhat")]
    {
        let s = dhat::HeapStats::get();
        [s.total_blocks, s.total_bytes, s.max_bytes as u64]
    }
    #[cfg(not(feature = "vec-dhat"))]
    [0; 3]
}
