//! THROWAWAY (ryo-fmx3.1 redo-alloc): the harness's global allocator, one cargo feature away from each other: jemalloc
//! (none given: what the command line, ruff and uv ship on Linux; redo-alloc cycle 1 measured it), glibc's (`vec-glibc`,
//! the allocator before), mimalloc (`vec-mimalloc`, as the workspace pins it), or dhat's counting heap profiler
//! (`vec-dhat`: every allocation counted with its call stack; slow, for counts only, never timed). The crate forbids
//! unsafe code, so the counter is dhat's, not a wrapper of ours.


#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;


/// The allocator's name, for the run's lines.
pub(super) const NAME: &str = if cfg!(feature = "vec-mimalloc") {
    "mimalloc"
} else if cfg!(feature = "vec-glibc") {
    "glibc"
} else if cfg!(feature = "vec-dhat") {
    "dhat"
} else {
    "jemalloc"
};

/// The run's profiler (`vec-dhat`; its file is `VEC_DHAT_OUT`, default `dhat-heap.json`), started at the run's start.
pub(super) struct Counter {
}

pub(super) fn start() -> Counter {
    Counter {
    }
}

/// (blocks allocated, bytes allocated, peak bytes live) so far; zeros without `vec-dhat`.
pub(super) fn counts() -> [u64; 3] {
        [0; 3]
}
