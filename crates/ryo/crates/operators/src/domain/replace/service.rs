//! Literal replacement over a borrowed prepared matcher.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use crate::domain::table_scratch::types::Patterns;
use crate::domain::table_texts::types::TextView;
use ryo_matching::inbound::use_cases::matching::types::{CompiledRegex, SearchText};
use ryo_values::domain::row_texts::types::CellText;
use std::ops::ControlFlow;

/// Visits the occurrences of the replaced pattern in the text, stopping when the callback breaks.
pub(crate) type VisitMatches<'visit> =
    &'visit dyn Fn(&mut dyn FnMut(std::ops::Range<usize>) -> ControlFlow<()>) -> ControlFlow<()>;

/// The text with every occurrence `visit` gives replaced by the replacement, written into the caller's scratch.
pub(crate) fn replace_compiled(
    visit: VisitMatches<'_>,
    text: CellText<'_>,
    replacement: ReplacementText<'_>,
    replaced: ReplacedText<'_>,
) -> Result<(), PortableOperatorFault> {
    let (text, replacement, output) = (text.0, replacement.0, replaced.0);
    output.clear();
    let mut end = 0;
    let mut failure = None;
    let _flow = visit(&mut |found| {
        let Some(prefix) = text.get(end..found.start) else {
            failure = Some(PortableOperatorFault::ShapeMismatch);
            return ControlFlow::Break(());
        };
        output.push_str(prefix);
        output.push_str(replacement);
        end = found.end;
        ControlFlow::Continue(())
    });
    if let Some(error) = failure {
        return Err(error);
    }
    output.push_str(
        text.get(end..)
            .ok_or(PortableOperatorFault::ShapeMismatch)?,
    );
    Ok(())
}

/// `replace`: the text with every occurrence of the pattern replaced by the replacement, written into the caller's
/// scratch. The pattern is compiled once into `compiled` and found again there; the occurrences are taken left to right
/// and do not overlap. The one meaning: today's `text_map` kernel and the new engine's `Call` both call it.
///
/// # Errors
///
/// `Pattern` when the pattern is no valid regular expression; `ShapeMismatch` when an occurrence does not lie in the
/// text after the one before it.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::patterns::types::PatternMatcher;
/// use ryo_operators::domain::replace::service::replaced;
/// use ryo_operators::domain::replace::types::{ReplacedText, ReplacementText};
/// use ryo_operators::domain::table_scratch::types::Patterns;
/// use ryo_operators::domain::table_texts::types::TextView;
/// use ryo_values::domain::row_texts::types::CellText;
///
/// let (mut kept, mut out) = (Patterns::default(), String::new());
/// replaced(
///     CellText("a-b-c"),
///     TextView("-"),
///     ReplacementText("_"),
///     &mut kept,
///     PatternMatcher(&MatchingUseCaseImpl),
///     ReplacedText(&mut out),
/// )
/// .expect("a valid expression");
/// assert_eq!(out, "a_b_c");
/// ```
pub fn replaced(
    text: CellText<'_>,
    pattern: TextView<'_>,
    replacement: ReplacementText<'_>,
    compiled: &mut Patterns,
    matcher: PatternMatcher<'_>,
    out: ReplacedText<'_>,
) -> Result<(), PortableOperatorFault> {
    let regex = crate::domain::patterns::service::compiled(pattern, compiled, matcher)?;
    replaced_by(regex, text, replacement, matcher, out)
}

/// `replace` with the pattern already compiled: for a caller that replaces in many texts, which compiles it once
/// (`patterns::service::compiled`) and not once a text.
///
/// # Errors
///
/// `ShapeMismatch` when an occurrence does not lie in the text after the one before it.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use ryo_operators::domain::patterns::{service::compiled, types::PatternMatcher};
/// use ryo_operators::domain::replace::service::replaced_by;
/// use ryo_operators::domain::replace::types::{ReplacedText, ReplacementText};
/// use ryo_operators::domain::table_scratch::types::Patterns;
/// use ryo_operators::domain::table_texts::types::TextView;
/// use ryo_values::domain::row_texts::types::CellText;
///
/// let (matcher, mut kept, mut out) = (PatternMatcher(&MatchingUseCaseImpl), Patterns::default(), String::new());
/// let regex = compiled(TextView("-"), &mut kept, matcher).expect("a valid expression");
/// for (text, wanted) in [("a-b", "a_b"), ("c", "c")] {
///     replaced_by(regex, CellText(text), ReplacementText("_"), matcher, ReplacedText(&mut out))
///         .expect("the occurrences lie in the text");
///     assert_eq!(out, wanted);
/// }
/// ```
pub fn replaced_by(
    regex: &CompiledRegex,
    text: CellText<'_>,
    replacement: ReplacementText<'_>,
    matcher: PatternMatcher<'_>,
    out: ReplacedText<'_>,
) -> Result<(), PortableOperatorFault> {
    replace_compiled(
        &|each| {
            matcher
                .0
                .visit_regex_matches(regex, SearchText(text.0), &mut |found| each(found.0))
        },
        text,
        replacement,
        out,
    )
}

/// The one lowering of a replacement: its text column, pattern and replacement, for the `text_map` kernel.
pub(crate) fn lower(
    text: ryo_values::domain::plan_parts::types::ColumnIndex,
    pattern: &ryo_values::domain::plan_parts::types::LiteralText,
    replacement: &ryo_values::domain::plan_parts::types::LiteralText,
) -> super::types::ReplaceParts {
    super::types::ReplaceParts {
        text,
        pattern: pattern.clone(),
        replacement: replacement.clone(),
    }
}
