//! The regex selector, `{ regex = "…" }`: a regular expression over the paths beneath the place it is written for. It
//! covers a path that matches and everything beneath it, as a glob does.

use super::super::interface::SelectorsItem;
use crate::domain::selectors::constants::EVERY_PATH;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::types::{
    BinderText, Binding, CompiledPattern, GlobForm, MatcherShape, PathMatcher, PathPattern,
    Repetition, Resolution, Resolved, SelectionContext, Selector, Spelling,
};

/// The regex selector kind.
pub(crate) struct RegexItem;

impl SelectorsItem for RegexItem {
    fn resolve(
        &self,
        selector: &Selector,
        context: &SelectionContext<'_>,
    ) -> Result<Resolution, SelectorsError> {
        let Selector::Regex(expression) = selector else {
            return Ok(Resolution::NotMine);
        };
        let spelling = Spelling {
            glob: EVERY_PATH.into(),
            binder: Binding::Expression(BinderText(expression.0.clone())),
            groups: Vec::new(),
        };
        let test =
            context
                .compile
                .compile(&spelling)
                .map_err(|failure| SelectorsError::Pattern {
                    pattern: expression.0.to_string(),
                    reason: failure.0.to_string(),
                })?;
        Ok(Resolution::Resolved(Resolved::Pattern(PathMatcher {
            text: PathPattern(expression.0.clone()),
            base: context.base.clone(),
            prefix: context.base.clone(),
            shape: MatcherShape::Compiled(CompiledPattern {
                test,
                groups: Vec::new(),
                repeats: Repetition::Once,
                glob: GlobForm::Bound,
            }),
            values: context.values.clone(),
        })))
    }
}
