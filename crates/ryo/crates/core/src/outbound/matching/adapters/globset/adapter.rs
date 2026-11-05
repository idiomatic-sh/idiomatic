//! The matching port, implemented with globset: every glob goes into one set, so a path is
//! matched against all of them in one pass.

use std::sync::Arc;

use globset::{GlobBuilder, GlobSetBuilder};

use ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::{GlobsMatcher, Hits, MatchCandidate};

/// The globset matching adapter.
#[derive(Debug)]
pub(crate) struct MatchingGlobsetAdapter;

impl MatchingPort for MatchingGlobsetAdapter {
    fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError> {
        let mut builder = GlobSetBuilder::new();
        let added = globs.iter().try_fold(&mut builder, |builder, glob| {
            let compiled = GlobBuilder::new(&glob.0)
                .literal_separator(true)
                .build()
                .map_err(|error| MatchingError::BadGlob {
                    glob: glob.0.to_string(),
                    reason: error.to_string(),
                })?;
            Ok::<&mut GlobSetBuilder, MatchingError>(builder.add(compiled))
        })?;
        let set = added.build().map_err(|error| MatchingError::BadGlob {
            glob: String::new(),
            reason: error.to_string(),
        })?;
        Ok(GlobsMatcher {
            matches: Arc::new(move |path: MatchCandidate<'_>, hits: &mut Hits| {
                set.matches_into(path.0, &mut hits.0);
            }),
        })
    }

    fn parse(&self, glob: &Glob) -> Result<(), MatchingError> {
        GlobBuilder::new(&glob.0)
            .literal_separator(true)
            .build()
            .map(|_| ())
            .map_err(|error| MatchingError::BadGlob {
                glob: glob.0.to_string(),
                reason: error.to_string(),
            })
    }
}
