//! The types of the `scheduler` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// How many workers the invocation's pool runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerCount(pub usize);

/// The threads the invocation's parallel work runs on: built once by the edge and handed to every consumer (the
/// scheduler and ryo's engine alike), or why the system refused them, which every run answers.
// ignore: idiomatic.arch/port/plain-port-types - a thread pool has no value to compare: two pools are the same only by identity, so the pool derives no `PartialEq`
#[derive(Clone, Debug)]
pub struct WorkerPool(pub Result<std::sync::Arc<rayon::ThreadPool>, super::errors::SchedulerError>);
