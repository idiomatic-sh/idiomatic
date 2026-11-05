//! Maps the injected engine port to the independent matching semantic contract.

use std::sync::Arc;

use ryo_execution::inbound::use_cases::execution::types::matching::{MatchingError, MatchingPort};
use ryo_matching::inbound::use_cases::matching::errors::MatchingError as SemanticError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_values::domain::files::types::{Glob, GlobsMatcher, Hits, MatchCandidate};

/// Adapts semantic results into the engine's caller-owned matcher handle.
#[derive(Debug)]
pub(crate) struct MatchingGlobsetAdapter;

impl MatchingPort for MatchingGlobsetAdapter {
    fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError> {
        let authored: Vec<&str> = globs.iter().map(|glob| glob.0.as_ref()).collect();
        let compiled = MatchingUseCaseImpl::new()
            .compile_globs(&authored)
            .map_err(matching_error)?;
        Ok(GlobsMatcher {
            matches: Arc::new(move |path: MatchCandidate<'_>, hits: &mut Hits| {
                MatchingUseCaseImpl::new().match_globs(&compiled, std::path::Path::new(path.0), &mut hits.0);
            }),
        })
    }

    fn parse(&self, glob: &Glob) -> Result<(), MatchingError> {
        MatchingUseCaseImpl::new()
            .parse_glob(&glob.0)
            .map_err(matching_error)
    }
}

/// Only glob requests enter this port; preserve the semantic refusal verbatim.
fn matching_error(error: SemanticError) -> MatchingError {
    match error {
        SemanticError::BadGlob { glob, reason } => MatchingError::BadGlob { glob, reason },
        SemanticError::InvalidRegex { reason } => MatchingError::BadGlob {
            glob: String::new(),
            reason,
        },
    }
}
