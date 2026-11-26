//! The wiring of `linter lint`: the edge takes the parts its caller supplies, chooses each implementation
//! in its builders, and builds the service it calls. `compose` only constructs. Nothing here implements a port.

use std::sync::Arc;

use ryo_core::outbound::{language, scheduler};
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_execution::outbound::matching;
use ryo_execution_runtime::outbound::digest;

use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::Language;

use super::types::LinterLintEngine;
use crate::domain::project::types::{DependencyBytes, DependencyFingerprint};

/// Chooses the parts every run over a linter shares, once: the languages linked, the compiler of queries and its
/// catalog, and the matching.
pub(crate) fn build_engine(languages: Vec<Language>, pool: WorkerPool, cache: super::types::Cache) -> LinterLintEngine {
    let workers = pool.workers();
    LinterLintEngine {
        scheduler: pool.of(),
        workers,
        cache,
        languages: crate::domain::language::types::Languages(languages),
        session: super::executor::NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    }
}

/// Builds the scheduler over the pool of threads runs read files on: `asked` of them, at most as many as the machine has, and as many
/// as it has when none is asked.
pub(crate) fn build_pool(
    workers: scheduler::types::WorkerCount,
) -> Arc<dyn scheduler::interface::SchedulerPort> {
    Arc::new(scheduler::factory::rayon(workers))
}

/// How many workers `build_pool(asked)` runs jobs on.
pub(crate) fn build_workers(
    asked: ryo_values::domain::runs::types::WorkersAsked,
) -> scheduler::types::WorkerCount {
    scheduler::types::WorkerCount(
        ryo_values::domain::runs::service::workers_to_start(
            asked,
            super::connections::machine_workers(),
        )
        .0,
    )
}


/// A linked language as the engine's language port takes it: its name, its build and its two use cases. What every edge of the SDK, and
/// the command line, composes the engine with: the use cases are reached through the engine, never through a method of the language.
#[doc(hidden)]
impl From<Language> for language::factory::LanguageConnection {
    fn from(linked: Language) -> Self {
        Self {
            name: ryo_values::domain::descriptions::types::LanguageName(linked.name().0.clone()),
            build: language::types::LanguageBuild(linked.build().0.clone()),
            parse: linked.parse_use_case(),
            project: linked.project_use_case(),
            walk: linked.walk_use_case(),
        }
    }
}

/// A pool the caller already runs, as the threads every run reads files on: one pool shared by the caller and the linter.
impl From<Arc<rayon::ThreadPool>> for WorkerPool {
    fn from(pool: Arc<rayon::ThreadPool>) -> Self {
        let workers = scheduler::types::WorkerCount(
            u32::try_from(pool.current_num_threads()).unwrap_or(u32::MAX),
        );
        Self::scheduler(Arc::new(scheduler::factory::rayon_over(pool)), workers)
    }
}

/// The dependency content identity using the digest adapter composed by the SDK.
impl From<DependencyBytes<'_>> for DependencyFingerprint {
    fn from(bytes: DependencyBytes<'_>) -> Self {
        use ryo_execution_runtime::outbound::digest::interface::DigestPort as _;
        let digest = digest::factory::blake3().digest(digest::types::DigestBytes(bytes.0));
        Self(digest.0)
    }
}


/// Chooses the matching a glob is checked with.
pub(crate) fn build_matching() -> impl MatchingPort {
    matching::factory::globset()
}
