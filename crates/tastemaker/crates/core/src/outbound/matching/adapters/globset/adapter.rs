//! The matching port, implemented with globset: the glob every path passes first, and the regex that binds captures and
//! reads `?` as one character.

use std::sync::Arc;

use crate::domain::captures::types::CaptureValue;
use crate::domain::selectors::types::{
    Binding, Coverage, GlobIndex, PathRest, PathSet, PathTest, PatternFailure, Spelling,
};
use crate::outbound::matching::interface::MatchingPort;

/// The globset matching adapter: each spelling compiled once, the first time it is asked for, and kept for the run: an
/// entry's pattern reads the same at every scope it is placed in.
#[derive(Debug, Default)]
pub(crate) struct MatchingGlobsetAdapter {
    /// What it has compiled so far, so a spelling is compiled once a run.
    compiled: Kept,
}

/// The spellings compiled so far, each with its test.
#[derive(Default)]
struct Kept(std::sync::Mutex<std::collections::HashMap<Spelling, Arc<dyn PathTest>>>);

impl std::fmt::Debug for Kept {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// A compiled spelling.
struct Compiled {
    /// A set of the one glob: a set matches by the glob's shape (a literal, an extension, a prefix) before falling back to
    /// a regular expression, which a lone matcher always runs.
    glob: globset::GlobSet,
    /// The spelling's regular expression over the path's captures, when it binds any.
    expression: Option<regex::Regex>,
}

impl PathTest for Compiled {
    fn passes(&self, rest: PathRest<'_>) -> Coverage {
        let rest = rest.0;
        let passes = self.glob.is_match(rest)
            && self
                .expression
                .as_ref()
                .is_none_or(|expression| expression.is_match(rest));
        if passes {
            Coverage::Covered
        } else {
            Coverage::Uncovered
        }
    }

    fn bind(&self, rest: PathRest<'_>) -> Option<Vec<CaptureValue>> {
        let rest = rest.0;
        if !self.glob.is_match(rest) {
            return None;
        }
        let Some(expression) = &self.expression else {
            return Some(Vec::new());
        };
        let found = expression.captures(rest)?;
        Some(
            found
                .iter()
                .skip(1)
                .map(|group| {
                    CaptureValue(group.map(|group| group.as_str()).unwrap_or_default().into())
                })
                .collect(),
        )
    }
}

/// Globs compiled into one set.
struct Together(globset::GlobSet);

impl PathSet for Together {
    fn passing(&self, rest: PathRest<'_>, into: &mut Vec<GlobIndex>) {
        let mut found = Vec::new();
        self.0.matches_into(rest.0, &mut found);
        into.clear();
        into.extend(found.into_iter().map(GlobIndex));
    }
}

impl MatchingPort for MatchingGlobsetAdapter {
    fn compile_set(
        &self,
        globs: &[crate::domain::selectors::types::SetGlob],
    ) -> Result<Arc<dyn PathSet>, PatternFailure> {
        let built = globs
            .iter()
            .map(|glob| {
                globset::GlobBuilder::new(&glob.0)
                    .literal_separator(true)
                    .backslash_escape(true)
                    .build()
                    .map_err(|error| PatternFailure(error.to_string().into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut builder = globset::GlobSetBuilder::new();
        let filled = built
            .into_iter()
            .fold(&mut builder, |builder, glob| builder.add(glob));
        let set = filled
            .build()
            .map_err(|error| PatternFailure(error.to_string().into()))?;
        Ok(Arc::new(Together(set)))
    }

    fn compile(&self, spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
        let kept = self
            .compiled
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(spelling)
            .cloned();
        if let Some(kept) = kept {
            return Ok(kept);
        }
        let compiled = Self::fresh(spelling)?;
        drop(
            self.compiled
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert(spelling.clone(), Arc::clone(&compiled)),
        );
        Ok(compiled)
    }
}

impl MatchingGlobsetAdapter {
    /// `spelling` compiled afresh.
    fn fresh(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
        let glob = globset::GlobBuilder::new(&spelling.glob)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map_err(|error| PatternFailure(error.to_string().into()))?;
        let glob = globset::GlobSetBuilder::new()
            .add(glob)
            .build()
            .map_err(|error| PatternFailure(error.to_string().into()))?;
        let expression = match &spelling.binder {
            Binding::GlobOnly => None,
            Binding::Expression(text) => Some(
                regex::Regex::new(text)
                    .map_err(|error| PatternFailure(error.to_string().into()))?,
            ),
        };
        Ok(Arc::new(Compiled { glob, expression }))
    }
}
