//! The fake of the `matching` port: globs of literal segments, `*` and `**`, matched without a library.

use std::sync::Arc;

use crate::domain::captures::types::CaptureValue;
use crate::domain::selectors::types::{
    Binding, Coverage, GlobIndex, PathRest, PathSet, PathTest, PatternFailure, Spelling,
};
use crate::outbound::matching::interface::MatchingPort;

/// What the core's tests stand in for `matching` with: no expression, no character class.
#[derive(Clone, Copy, Debug, Default)]
pub struct MatchingFake;

/// A glob split into its segments.
struct Segments(Vec<String>);

/// Whether `segments` of a glob match `parts` of a path.
fn fits(segments: &[String], parts: &[&str]) -> bool {
    match (segments.split_first(), parts.split_first()) {
        (None, None) => true,
        (Some((first, rest)), _) if first == "**" => {
            (0..=parts.len()).any(|skip| parts.get(skip..).is_some_and(|tail| fits(rest, tail)))
        }
        (Some((first, rest)), Some((part, others))) => {
            fits_segment(first, part) && fits(rest, others)
        }
        _ => false,
    }
}

/// Whether one glob segment, literal text and `*`, matches one path segment.
fn fits_segment(glob: &str, part: &str) -> bool {
    match glob.split_once('*') {
        None => glob == part,
        Some((before, after)) => part.strip_prefix(before).is_some_and(|rest| {
            (0..=rest.len())
                .filter(|at| rest.is_char_boundary(*at))
                .any(|at| rest.get(at..).is_some_and(|tail| fits_segment(after, tail)))
        }),
    }
}

impl PathTest for Segments {
    fn passes(&self, rest: PathRest<'_>) -> Coverage {
        if fits(&self.0, &rest.0.split('/').collect::<Vec<_>>()) {
            Coverage::Covered
        } else {
            Coverage::Uncovered
        }
    }

    fn bind(&self, rest: PathRest<'_>) -> Option<Vec<CaptureValue>> {
        (self.passes(rest) == Coverage::Covered).then(Vec::new)
    }
}

/// Globs matched one after the other.
struct Each(Vec<Segments>);

impl PathSet for Each {
    fn passing(&self, rest: PathRest<'_>, into: &mut Vec<GlobIndex>) {
        into.clear();
        into.extend(
            self.0
                .iter()
                .enumerate()
                .filter(|(_, glob)| glob.passes(rest) == Coverage::Covered)
                .map(|(place, _)| GlobIndex(place)),
        );
    }
}

impl MatchingPort for MatchingFake {
    fn compile_set(
        &self,
        globs: &[crate::domain::selectors::types::SetGlob],
    ) -> Result<Arc<dyn PathSet>, PatternFailure> {
        let globs = globs.iter().map(|glob| glob.0.clone()).collect::<Vec<_>>();
        if globs.iter().any(|glob| glob.contains(['[', '{', '?'])) {
            return Err(PatternFailure(
                "the fake reads literal segments, `*` and `**` only".into(),
            ));
        }
        Ok(Arc::new(Each(
            globs
                .iter()
                .map(|glob| Segments(glob.split('/').map(str::to_owned).collect()))
                .collect(),
        )))
    }

    fn compile(&self, spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
        match &spelling.binder {
            Binding::GlobOnly if !spelling.glob.contains(['[', '{', '?']) => Ok(Arc::new(
                Segments(spelling.glob.split('/').map(str::to_owned).collect()),
            )),
            Binding::GlobOnly | Binding::Expression(_) => Err(PatternFailure(
                "the fake reads literal segments, `*` and `**` only".into(),
            )),
        }
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
