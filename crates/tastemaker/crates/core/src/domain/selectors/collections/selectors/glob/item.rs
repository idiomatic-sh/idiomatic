//! The glob selector: a path pattern, written as a bare string. It is spelled here into the glob and the binding
//! expression the matching port compiles: `*` one segment, `**` any depth, `?` one character, `{name}` a capture within a
//! segment (never empty), `{*name}` one or more whole segments, `{name:dir}` a capture within the last segment of a
//! template entry's `file` whose value names a directory beside the file; a name with a value is interpolated instead. A
//! `\\` makes the character after it stand for itself (the captures concept's grammar, `pieces`).

use std::collections::BTreeSet;

use super::super::interface::SelectorsItem;
use crate::domain::captures::constants::MARK_OPEN;
use crate::domain::captures::service::{captured_names, pieces};
use crate::domain::captures::types::{CaptureName, Interpolations, PatternPiece, Template};
use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath, Segment};
use crate::domain::selectors::constants::{
    ANY_DEPTH, ANY_DEPTH_LEADING, ANY_DEPTH_MIDDLE, ANY_DEPTH_TRAILING, BASE_ITSELF,
    CAPTURE_SEGMENT, CAPTURE_SEGMENTS, DIRECTORY_IN_SELECTOR, DIRECTORY_NOT_LAST, ONE_CHARACTER,
    REGEX_SPECIALS, UNKNOWN_MARK, WHOLE_DEPTH, WHOLE_DEPTH_LEADING, WHOLE_DEPTH_MIDDLE,
    WHOLE_DEPTH_TRAILING, WITHIN_SEGMENT,
};
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::types::{
    BinderText, Binding, CompiledPattern, GlobForm, MatcherShape, PathMatcher, PathPattern,
    PatternCompiler, Repetition, Resolution, Resolved, SelectionContext, Selector, SpelledGlob,
    Spelling,
};

/// The glob selector kind.
pub(crate) struct GlobItem;

impl SelectorsItem for GlobItem {
    fn resolve(
        &self,
        selector: &Selector,
        context: &SelectionContext<'_>,
    ) -> Result<Resolution, SelectorsError> {
        let Selector::Glob(pattern) = selector else {
            return Ok(Resolution::NotMine);
        };
        // A selector is a path test that reads no workspace: a `{name:dir}` would silently mean `{name}` in it.
        if pieces(Template(&pattern.0))
            .iter()
            .any(|piece| matches!(piece, PatternPiece::Directory(_)))
        {
            return Err(SelectorsError::Pattern {
                pattern: pattern.0.to_string(),
                reason: DIRECTORY_IN_SELECTOR.to_owned(),
            });
        }
        place(context.base, pattern, context.values, context.compile)
            .map(|matcher| Resolution::Resolved(Resolved::Pattern(matcher)))
    }
}

/// The values among `values` the pattern interpolates: those of the names it writes as `{name}`, `{*name}` or
/// `{name:dir}`.
#[must_use]
pub(crate) fn interpolated(pattern: &PathPattern, values: &Interpolations) -> Interpolations {
    let mut found = Interpolations::default();
    for name in captured_names(Template(&pattern.0)) {
        if let Some(value) = values.0.get(&name) {
            drop(found.0.insert(name, value.clone()));
        }
    }
    found
}

/// `pattern` placed at `base` with `values` interpolated, compiled through `compile`, with the `{name:dir}` captures of
/// its last segment.
///
/// # Errors
///
/// `Pattern` when the spelled glob or expression does not compile, a capture's name holds a `:` other than the one
/// `:dir` mark, or a `{name:dir}` is written before the last segment.
pub(crate) fn place(
    base: &RepoPath,
    pattern: &PathPattern,
    values: &Interpolations,
    compile: &dyn PatternCompiler,
) -> Result<PathMatcher, SelectorsError> {
    let values = interpolated(pattern, values);
    if &*pattern.0 == BASE_ITSELF {
        return Ok(PathMatcher {
            text: pattern.clone(),
            base: base.clone(),
            prefix: base.clone(),
            shape: MatcherShape::Itself,
            values,
            directories: Vec::new(),
        });
    }
    marks(pattern)?;
    let directories = directory_captures(pattern)?;
    let Spelled { spelling, prefix } = spell(pattern, &values);
    let repeats = if spelling.groups.iter().collect::<BTreeSet<_>>().len() == spelling.groups.len()
    {
        Repetition::Once
    } else {
        Repetition::Repeated
    };
    let test = compile
        .compile(&spelling)
        .map_err(|failure| SelectorsError::Pattern {
            pattern: pattern.0.to_string(),
            reason: failure.0.to_string(),
        })?;
    let prefix = prefix.iter().fold(base.clone(), |prefix, segment| {
        join(&prefix, PathText(&segment.0)).unwrap_or(prefix)
    });
    Ok(PathMatcher {
        text: pattern.clone(),
        base: base.clone(),
        prefix,
        shape: MatcherShape::Compiled(CompiledPattern {
            test,
            glob: match spelling.binder {
                Binding::GlobOnly => GlobForm::Alone(spelling.glob),
                Binding::Expression(_) => GlobForm::Bound,
            },
            groups: spelling.groups,
            repeats,
        }),
        values,
        directories,
    })
}

/// Refuses a capture whose name holds a `:`: `{*name:dir}` and `{name:dirs}` are misspelt marks, never plain captures.
///
/// # Errors
///
/// `Pattern` when one does.
fn marks(pattern: &PathPattern) -> Result<(), SelectorsError> {
    let misspelt = pieces(Template(&pattern.0))
        .iter()
        .any(|piece| match piece {
            PatternPiece::Capture(name)
            | PatternPiece::Span(name)
            | PatternPiece::Directory(name) => name.0.contains(MARK_OPEN),
            PatternPiece::Literal(_) | PatternPiece::Star | PatternPiece::Question => false,
        });
    if misspelt {
        return Err(SelectorsError::Pattern {
            pattern: pattern.0.to_string(),
            reason: UNKNOWN_MARK.to_owned(),
        });
    }
    Ok(())
}

/// The `{name:dir}` captures of the pattern's last segment, the one that names the file.
///
/// # Errors
///
/// `Pattern` when one is written in an earlier segment, which has no directory beside it to name.
fn directory_captures(pattern: &PathPattern) -> Result<Vec<CaptureName>, SelectorsError> {
    let mut segments = pattern.0.rsplit('/');
    let last = segments.next().unwrap_or_default();
    let directory = |piece: &PatternPiece<'_>| match piece {
        PatternPiece::Directory(name) => Some(CaptureName(name.0.into())),
        PatternPiece::Literal(_)
        | PatternPiece::Star
        | PatternPiece::Question
        | PatternPiece::Capture(_)
        | PatternPiece::Span(_) => None,
    };
    if segments.any(|segment| {
        pieces(Template(segment))
            .iter()
            .any(|piece| directory(piece).is_some())
    }) {
        return Err(SelectorsError::Pattern {
            pattern: pattern.0.to_string(),
            reason: DIRECTORY_NOT_LAST.to_owned(),
        });
    }
    Ok(pieces(Template(last))
        .iter()
        .filter_map(directory)
        .collect())
}

/// The pattern as it is being spelled.
#[derive(Default)]
struct Writing {
    /// The glob text written so far.
    glob: Draft,
    /// The expression text written so far.
    expression: Draft,
    /// The captures the expression groups, in order.
    groups: Vec<CaptureName>,
    /// Whether the pattern holds a `?`.
    questions: Questions,
}

/// The text of a glob or an expression as it is being written.
#[derive(Default)]
struct Draft(String);

/// Text of a pattern matched as written.
#[derive(Clone, Copy)]
struct Verbatim<'a>(&'a str);

/// A spelled pattern, and the literal directories it starts with.
pub(crate) struct Spelled {
    /// The spelling.
    pub spelling: Spelling,
    /// The literal directories.
    pub prefix: Vec<Segment>,
}

/// Whether a spelled pattern holds a `?`, which globset reads as one byte and the expression as one character.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Questions {
    /// The pattern holds no `?`.
    #[default]
    None,
    /// The pattern holds a `?`.
    Some,
}

impl Writing {
    /// Text matched as written.
    fn literal(&mut self, text: Verbatim<'_>) {
        for character in text.0.chars() {
            match character {
                '*' | '?' | '[' | ']' | '{' | '}' | '\\' => {
                    self.glob.0.push('[');
                    self.glob.0.push(character);
                    self.glob.0.push(']');
                }
                other => self.glob.0.push(other),
            }
        }
        escape(text, &mut self.expression);
    }
}

/// Writes `text` into `into` with every character a regular expression gives a meaning escaped.
fn escape(text: Verbatim<'_>, into: &mut Draft) {
    for character in text.0.chars() {
        if REGEX_SPECIALS.contains(character) {
            into.0.push('\\');
        }
        into.0.push(character);
    }
}

/// The spelling of `pattern` with `values` interpolated, and the literal directories it starts with.
#[must_use]
pub(crate) fn spell(pattern: &PathPattern, values: &Interpolations) -> Spelled {
    // `**/**` means what one `**` means.
    let mut segments: Vec<&str> = Vec::new();
    for segment in pattern.0.split('/') {
        if segment == ANY_DEPTH && segments.last() == Some(&ANY_DEPTH) {
            continue;
        }
        segments.push(segment);
    }
    let mut writing = Writing::default();
    let mut prefix = Vec::new();
    let mut literal = true;
    let mut after_separator = true;
    let last = segments.len().saturating_sub(1);
    for (index, segment) in segments.iter().enumerate() {
        if *segment == ANY_DEPTH {
            literal = false;
            // globset's `**`: a leading `**/` is any depth, a middle `/**/` too, a trailing `/**` at least one
            // segment, and `**` alone everything.
            let (glob, expression) = match (index == 0, index == last) {
                (true, true) => (ANY_DEPTH, WHOLE_DEPTH),
                (true, false) => (ANY_DEPTH_LEADING, WHOLE_DEPTH_LEADING),
                (false, true) => (ANY_DEPTH_TRAILING, WHOLE_DEPTH_TRAILING),
                (false, false) => (ANY_DEPTH_MIDDLE, WHOLE_DEPTH_MIDDLE),
            };
            writing.glob.0.push_str(glob);
            writing.expression.0.push_str(expression);
            after_separator = index != last;
            continue;
        }
        if !after_separator {
            writing.glob.0.push('/');
            writing.expression.0.push('/');
        }
        after_separator = false;
        let text = match span_name(Verbatim(segment)) {
            Some(name) => spell_span(&name, values, &mut writing),
            None => spell_segment(Verbatim(segment), values, &mut writing),
        };
        if literal {
            match text {
                Some(text) => prefix.push(text),
                None => literal = false,
            }
        }
    }
    let binder = if writing.groups.is_empty() && writing.questions == Questions::None {
        Binding::GlobOnly
    } else {
        Binding::Expression(BinderText(format!("^{}$", writing.expression.0).into()))
    };
    Spelled {
        spelling: Spelling {
            glob: SpelledGlob(writing.glob.0.into()),
            binder,
            groups: writing.groups,
        },
        prefix,
    }
}

/// The name of a segment written `{*name}`, alone in its segment.
fn span_name(segment: Verbatim<'_>) -> Option<CaptureName> {
    match pieces(Template(segment.0)).as_slice() {
        [PatternPiece::Span(name)] => Some(CaptureName(name.0.into())),
        _ => None,
    }
}

/// Spells a `{*name}` segment: its value as written when it has one, else a capture of one or more whole segments.
fn spell_span(
    key: &CaptureName,
    values: &Interpolations,
    writing: &mut Writing,
) -> Option<Segment> {
    if let Some(value) = values.0.get(key) {
        writing.literal(Verbatim(&value.0));
        return Some(Segment(value.0.clone()));
    }
    // globset's `**` passes every candidate, zero segments too; the binder then requires one or more.
    writing.glob.0.push_str(ANY_DEPTH);
    writing.expression.0.push_str(CAPTURE_SEGMENTS);
    writing.groups.push(key.clone());
    None
}

/// Spells one segment: literal text (escaped characters among it) and interpolated values as written, `*` and `?` as
/// wildcards, and each other `{name}` as a capture. Answers the segment's text when it has neither wildcard nor capture.
fn spell_segment(
    segment: Verbatim<'_>,
    values: &Interpolations,
    writing: &mut Writing,
) -> Option<Segment> {
    let mut text = Some(String::new());
    for piece in pieces(Template(segment.0)) {
        match piece {
            PatternPiece::Capture(name)
            | PatternPiece::Span(name)
            | PatternPiece::Directory(name) => {
                let name = CaptureName(name.0.into());
                if let Some(value) = values.0.get(&name) {
                    writing.literal(Verbatim(&value.0));
                    if let Some(text) = text.as_mut() {
                        text.push_str(&value.0);
                    }
                } else {
                    writing.glob.0.push('*');
                    writing.expression.0.push_str(CAPTURE_SEGMENT);
                    writing.groups.push(name);
                    text = None;
                }
            }
            PatternPiece::Star => {
                writing.glob.0.push('*');
                writing.expression.0.push_str(WITHIN_SEGMENT);
                text = None;
            }
            // globset's `?` is one byte; `*` lets the glob pass every candidate the binder then decides by character.
            PatternPiece::Question => {
                writing.glob.0.push('*');
                writing.expression.0.push_str(ONE_CHARACTER);
                writing.questions = Questions::Some;
                text = None;
            }
            PatternPiece::Literal(other) => {
                writing.literal(Verbatim(&other.0.to_string()));
                if let Some(text) = text.as_mut() {
                    text.push(other.0);
                }
            }
        }
    }
    text.map(|text| Segment(text.into()))
}
