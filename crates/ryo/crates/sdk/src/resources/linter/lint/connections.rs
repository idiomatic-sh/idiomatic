//! The connections of `linter lint`: it opens the stores the edge reads and writes and returns their
//! clients. This is the only I/O of the edge; the composition takes the clients it returns and only constructs.

use ryo_core::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};

use crate::domain::language::types::Language;

use super::errors::LintError;
use super::types::{Cache, Port, Spill};

/// The clients of the stores `linter lint` reaches, opened once: where the run keeps what it leaves for the next and which build
/// keeps it, none for a run that keeps nothing, and where it spills while it runs.
pub(crate) struct LinterLintConnections {
    /// The store of the run, in the caller's cache directory, for this build of the program and of its languages.
    pub store: Option<Store>,
    /// Where the run spills.
    pub spill: Spill,
}

/// Opens the stores and returns their clients: the caller's cache directory, made when no run has made it yet, since a
/// run spills there before it keeps its store; or none for a run that keeps nothing, which spills in the system's temporary directory.
///
/// # Errors
///
/// `Io` when the cache directory cannot be made.
pub(crate) fn connect(
    cache: &Cache,
    engine: &super::types::LinterLintEngine,
) -> Result<LinterLintConnections, LintError> {
    match cache {
        Cache::At(path) => {
            std::fs::create_dir_all(path).map_err(|error| LintError::Io {
                what: Port::Store,
                reason: error.to_string(),
            })?;
            Ok(LinterLintConnections {
                store: Some(Store {
                    directory: CacheDirectory(path.to_string_lossy().into()),
                    build: engine.build.clone(),
                }),
                spill: Spill::Kept(path.clone()),
            })
        }
        Cache::None => Ok(LinterLintConnections {
            store: None,
            spill: Spill::Scratch,
        }),
    }
}

/// Which build runs: the size and the date of the program say which build of it, the SDK in it included, and each linked language
/// says its own.
pub(crate) fn build(languages: &[Language]) -> BuildIdentity {
    let program = std::env::current_exe()
        .ok() // ignore: idiomatic.arch/no-ok-on-result - a program the system cannot describe has no size or time: the build identity reads 0
        .and_then(|path| std::fs::metadata(path).ok()); // ignore: idiomatic.arch/no-ok-on-result - a program the system cannot describe has no size or time: the build identity reads 0
    let modified = program
        .as_ref()
        .and_then(|meta| meta.modified().ok()) // ignore: idiomatic.arch/no-ok-on-result - a program the system cannot describe has no size or time: the build identity reads 0
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()) // ignore: idiomatic.arch/no-ok-on-result - a program the system cannot describe has no size or time: the build identity reads 0
        .map_or(0, |since| since.as_nanos());
    let size = program.map_or(0, |meta| meta.len());
    let mut build = size.to_string();
    let parts = [modified.to_string()];
    let linked = languages.iter().flat_map(|language| {
        [
            language.name().0.to_string(),
            language.build().0.to_string(),
            {
                let mut capabilities: Vec<_> = language
                    .parse_use_case()
                    .capabilities()
                    .iter()
                    .map(|capability| {
                        ryo_language::domain::relations::service::name(
                            ryo_language::domain::relations::service::capability_relation(
                                *capability,
                            ),
                        )
                        .0
                    })
                    .collect();
                capabilities.sort_unstable();
                capabilities.dedup();
                // Release identity plus length-delimited canonical names: declaration order is irrelevant.
                let mut identity = language.build().0.to_string();
                for capability in capabilities {
                    identity.push(' ');
                    identity.push_str(&capability.len().to_string());
                    identity.push(':');
                    identity.push_str(capability);
                }
                identity
            },
        ]
    });
    for part in parts.into_iter().chain(linked) {
        build.push(' ');
        build.push_str(&part);
    }
    BuildIdentity(build.into())
}
