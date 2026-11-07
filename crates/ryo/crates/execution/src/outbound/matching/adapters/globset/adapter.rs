//! Maps the injected engine port to the independent matching semantic contract.

use std::sync::Arc;

use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{GlobHits, GlobPattern};
use ryo_values::domain::files::types::{Glob, GlobsMatcher, Hits, MatchCandidate};

/// Adapts semantic results into the engine's caller-owned matcher handle.
#[derive(Debug)]
pub(crate) struct MatchingGlobsetAdapter<Semantics> {
    /// The matching semantics the edge built, shared with every matcher this adapter compiles.
    semantics: Arc<Semantics>,
}

impl<Semantics> MatchingGlobsetAdapter<Semantics> {
    /// The adapter over the matching semantics the edge built.
    pub(crate) fn new(semantics: Semantics) -> Self {
        Self {
            semantics: Arc::new(semantics),
        }
    }
}

impl<Semantics> MatchingPort for MatchingGlobsetAdapter<Semantics>
where
    Semantics: MatchingUseCase + Send + Sync + 'static,
{
    fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError> {
        let authored: Vec<GlobPattern<'_>> = globs
            .iter()
            .map(|glob| GlobPattern(glob.0.as_ref()))
            .collect();
        let compiled = self
            .semantics
            .compile_globs(&authored)
            .map_err(matching_error)?;
        let semantics = Arc::clone(&self.semantics);
        Ok(GlobsMatcher {
            matches: Arc::new(move |path: MatchCandidate<'_>, hits: &mut Hits| {
                let mut found = GlobHits(std::mem::take(&mut hits.0));
                semantics.match_globs(
                    &compiled,
                    std::path::Path::new(path.0),
                    &mut found,
                );
                hits.0 = found.0;
            }),
        })
    }

    fn parse(&self, glob: &Glob) -> Result<(), MatchingError> {
        self.semantics
            .parse_glob(GlobPattern(&glob.0))
            .map_err(matching_error)
    }
}

/// Only glob requests enter this port; preserve the semantic refusal verbatim.
fn matching_error(error: MatchingError) -> MatchingError {
    match error {
        MatchingError::BadGlob { glob, reason } => MatchingError::BadGlob { glob, reason },
        MatchingError::InvalidRegex { reason } => MatchingError::BadGlob {
            glob: String::new(),
            reason,
        },
    }
}
