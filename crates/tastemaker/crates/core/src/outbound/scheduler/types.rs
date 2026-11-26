//! The types of the `scheduler` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// How many workers the invocation's pool runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerCount(pub usize);

/// The pool size the edge set before the command ran, if it set one.
static CONFIGURED: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

impl WorkerCount {
    /// Sets the pool size the commands of this process run with, once, before any command builds its pool; a later call changes nothing.
    /// It answers whether this call set it. A host that wants a fixed pool (a measurement on one worker) calls it from its main.
    pub fn configure(count: usize) -> bool {
        CONFIGURED.set(count.max(1)).is_ok()
    }

    /// The pool size a command builds its pool with: the one `configure` set, else the CPUs the process may use. The commands read it
    /// here and nowhere else.
    #[must_use]
    pub fn configured() -> Self {
        Self(
            CONFIGURED
                .get()
                .copied()
                .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, std::num::NonZero::get)),
        )
    }
}

/// The threads the invocation's parallel work runs on: built once by the edge and handed to every consumer (the
/// scheduler and ryo's engine alike), or why the system refused them, which every run answers.
// ignore: idiomatic.arch/port/plain-port-types - a thread pool has no value to compare: two pools are the same only by identity, so the pool derives no `PartialEq`
#[derive(Clone, Debug)]
pub struct WorkerPool(pub Result<std::sync::Arc<rayon::ThreadPool>, super::errors::SchedulerError>);
