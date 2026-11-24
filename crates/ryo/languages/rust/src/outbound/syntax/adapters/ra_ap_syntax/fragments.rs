//! Rust provider 0.1.4 parses the direct arguments of a standard macro one fragment at a time.
//! Fragment boundaries determine expansion ordinals and the node identities in released rows.

use super::adapter::{index, offset, read_fragment};
use super::constants::{MATCHES, VEC};
use super::types::{MacroArgument, MacroName, RustText, Step};
use ra_ap_parser::TopEntryPoint;
use ra_ap_syntax::SyntaxKind;
use ryo::languages::{Count, Offset};

/// A direct child of a token tree: the bytes it spans, and its kind.
#[derive(Clone, Copy, Debug)]
struct Child {
    /// Where it starts.
    pub from: Offset,
    /// Where it ends.
    pub to: Offset,
    /// Its kind (that of its node, or of its token).
    pub kind: SyntaxKind,
}

/// A stretch of text in an argument list.
#[derive(Clone, Copy, Debug)]
struct ArgumentSpan {
    /// Where it starts.
    pub from: Offset,
    /// Where it ends.
    pub to: Offset,
}

/// What a stretch of an argument list is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FragmentEntryPoint {
    /// An expression.
    Expression,
    /// A pattern.
    Pattern,
}

/// A stretch of an argument list and what it is read as.
#[derive(Clone, Copy, Debug)]
struct Stretch {
    /// The text.
    pub span: ArgumentSpan,
    /// What it is read as.
    pub reading: FragmentEntryPoint,
}

/// The direct children of the token tree held in `steps`, between its delimiters, with the offsets from `start`; none when the steps
/// are not a clean token tree.
fn children_of(steps: &[Step], start: Offset) -> Option<Vec<Child>> {
    let [
        Step::Enter(SyntaxKind::TOKEN_TREE),
        Step::Token(_, open),
        inside @ ..,
        Step::Token(_, _),
        Step::Exit,
    ] = steps
    else {
        return None;
    };
    let mut found = Vec::new();
    let mut at = index(start).0 + open.0;
    let mut steps_inside = inside.iter();
    while let Some(step) = steps_inside.next() {
        match step {
            Step::Token(kind, length) => {
                found.push(Child {
                    from: offset(Count(at)),
                    to: offset(Count(at + length.0)),
                    kind: *kind,
                });
                at += length.0;
            }
            Step::Enter(kind) => {
                // The child node runs to the exit that closes this enter; a node left open is no clean token tree.
                let (from, mut depth) = (at, 1_usize);
                while depth > 0 {
                    match steps_inside.next()? {
                        Step::Enter(_) => depth += 1,
                        Step::Exit => depth -= 1,
                        Step::Token(_, length) => at += length.0,
                        Step::Error(..) => return None,
                    }
                }
                found.push(Child {
                    from: offset(Count(from)),
                    to: offset(Count(at)),
                    kind: *kind,
                });
            }
            Step::Exit | Step::Error(..) => return None,
        }
    }
    Some(found)
}

/// The bytes a stretch of children spans, without the white space at its ends.
fn trimmed(children: &[Child]) -> Option<ArgumentSpan> {
    let first = children
        .iter()
        .find(|each| !matches!(each.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT))?;
    let last = children
        .iter()
        .rev()
        .find(|each| !matches!(each.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT))?;
    Some(ArgumentSpan {
        from: first.from,
        to: last.to,
    })
}

/// The stretches of the arguments of the macro `name` that are read again, each with what it is read as.
fn stretches(name: &MacroName, children: &[Child]) -> Option<Vec<Stretch>> {
    let (vec, matches) = (**name == *VEC, **name == *MATCHES);
    let mut found = Vec::new();
    let mut place = 0_i32;
    for argument in children
        .split(|each| each.kind == SyntaxKind::COMMA || (vec && each.kind == SyntaxKind::SEMICOLON))
    {
        let Some(mut span) = trimmed(argument) else {
            continue;
        };
        // A named argument, `width = 3`, is read from its value.
        if !vec && !matches {
            let mut words = argument
                .iter()
                .filter(|each| !matches!(each.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT));
            if let (Some(label), Some(equals)) = (words.next(), words.next())
                && label.kind == SyntaxKind::IDENT
                && equals.kind == SyntaxKind::EQ
            {
                let after = argument
                    .iter()
                    .position(|each| std::ptr::eq(each, equals))?
                    + 1;
                span.from = trimmed(argument.get(after..)?)?.from;
            }
        }
        if matches && place >= 1_i32 {
            // A pattern, then perhaps `if` and a guard that is an expression.
            match argument
                .iter()
                .position(|each| each.kind == SyntaxKind::IF_KW)
            {
                Some(at) => {
                    found.push(Stretch {
                        span: trimmed(argument.get(..at)?)?,
                        reading: FragmentEntryPoint::Pattern,
                    });
                    found.push(Stretch {
                        span: trimmed(argument.get(at + 1..)?)?,
                        reading: FragmentEntryPoint::Expression,
                    });
                }
                None => found.push(Stretch {
                    span,
                    reading: FragmentEntryPoint::Pattern,
                }),
            }
        } else {
            found.push(Stretch {
                span,
                reading: FragmentEntryPoint::Expression,
            });
        }
        place += 1_i32;
    }
    Some(found)
}

/// The arguments of the macro `name` read again, in the token tree held in `steps` that starts at `start` of `text`: each stretch of
/// text with the steps it parses to; nothing when they are not all expressions (or patterns) of the macro's grammar.
pub(super) fn expanded(
    name: &MacroName,
    text: RustText<'_>,
    start: Offset,
    steps: &[Step],
) -> Option<Vec<MacroArgument>> {
    let children = children_of(steps, start)?;
    let mut out = Vec::new();
    for stretch in stretches(name, &children)? {
        let ArgumentSpan { from, to } = stretch.span;
        out.push(MacroArgument {
            from,
            steps: read_fragment(
                RustText(text.0.get(index(from).0..index(to).0)?),
                &match stretch.reading {
                    FragmentEntryPoint::Expression => TopEntryPoint::Expr,
                    FragmentEntryPoint::Pattern => TopEntryPoint::Pattern,
                },
                |step| matches!(step, Step::Error(..)),
            )?,
        });
    }
    Some(out)
}
