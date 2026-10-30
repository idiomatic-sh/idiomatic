//! The syntax port, implemented with rust-analyzer's parser: its events are written straight into
//! the element table, with no syntax tree built. The library's names stop here; what leaves is
//! the language's own `Kind`.
//!
//! The token tree of a call of one of the standard expression macros (`println!`, `assert_eq!`, `vec!`, ...) is read again as the
//! expressions it holds: the parser gives a token tree no structure, so the arguments are parsed as expressions (the pattern of
//! `matches!` as a pattern). The file's own tree keeps the token tree as the parser gave it; the expressions are trees added after it
//! under the macro call (`adopt`), which only the readers that ask for them see. A call that does not parse as the macro's grammar
//! adds nothing.

use super::constants::{CRLF, LF, MATCHES, VEC};
use super::schemas::kind_of;
use super::schemas::repaired;
use super::schemas::step_of;
use super::types::{
    ArgumentSpan, Child, Cursor, DecodedString, Expansion, Held, MacroArgument, MacroName, Open,
    Reading, RustText, Step, Stretch, StringLiteral, Triviality,
};
use crate::domain::syntax::service::kind_bits;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{Expanding, SourceText};
use ra_ap_parser::{LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::{Edition, SyntaxKind};
use rustc_literal_escaper::unescape_str;
use ryo::languages::{self, Count, Offset, Position};
/// The `ra_ap_syntax` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRaApSyntaxAdapter {
    /// The macros whose arguments are read again as the expressions they hold.
    expanding: Expanding<'static>,
}

impl SyntaxRaApSyntaxAdapter {
    /// The adapter, reading again the arguments of the macros `expanding` names.
    pub(crate) const fn new(expanding: Expanding<'static>) -> Self {
        Self { expanding }
    }
}

/// The value a string literal stands for, with its escapes resolved; nothing where an escape is wrong. A raw string is its text between
/// its delimiters, a carriage return and a line feed in it being one line feed.
fn decoded(literal: StringLiteral<'_>) -> Option<DecodedString> {
    let literal = literal.0;
    if let Some(raw) = literal.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let inner = raw.get(hashes + 1..raw.len().checked_sub(hashes + 1)?)?;
        return Some(DecodedString(inner.replace(CRLF, LF)));
    }
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut value = String::with_capacity(inner.len());
    let mut wrong = false;
    unescape_str(inner, |_, character| match character {
        Ok(each) => value.push(each),
        Err(_) => wrong = true,
    });
    (!wrong).then_some(DecodedString(value))
}

/// Writes `step` into the table at the cursor, which moves past the text of a token.
fn emit(
    builder: &mut languages::TableBuilder<'_>,
    text: RustText<'_>,
    step: &Step,
    cursor: &mut Cursor,
) {
    match step {
        Step::Enter(kind) => {
            cursor.next.0 += 1;
            languages::enter_node(builder, kind_bits(kind_of(*kind)), cursor.position);
        }
        Step::Token(kind, length) => {
            cursor.next.0 += 1;
            let end = Offset(
                cursor
                    .position
                    .0
                    .saturating_add(u32::try_from(length.0).unwrap_or(u32::MAX)),
            );
            let (from, to) = (index(cursor.position).0, index(end).0);
            let value = (*kind == SyntaxKind::STRING)
                .then(|| {
                    let written = text.0.get(from..to)?;
                    decoded(StringLiteral(written))
                })
                .flatten();
            match value {
                Some(value) => languages::decoded_token(
                    builder,
                    kind_bits(kind_of(*kind)),
                    cursor.position,
                    end,
                    languages::NameText(&value.0),
                ),
                None => languages::token(builder, kind_bits(kind_of(*kind)), cursor.position, end),
            }
            cursor.position = end;
        }
        Step::Exit => languages::leave_node(builder, cursor.position),
        Step::Error(message, at) => {
            languages::diagnose(builder, languages::ReportText(message), *at, *at);
        }
    }
}

impl SyntaxPort for SyntaxRaApSyntaxAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let repaired = repaired(text);
        let whole: &str = &repaired.0;
        let expanding = self.expanding;
        let lexed = LexedStr::new(Edition::CURRENT, whole);
        let input = lexed.to_input(Edition::CURRENT);
        let output = TopEntryPoint::SourceFile.parse(&input);
        for (error, message) in lexed.errors() {
            let range = lexed.text_range(error);
            languages::diagnose(
                builder,
                languages::ReportText(message),
                offset(Count(range.start)),
                offset(Count(range.end)),
            );
        }
        let mut cursor = Cursor {
            position: Offset(0),
            next: Position(0),
        };
        // The last name written before a `!`, and whether the token tree that follows belongs to a call of an expression macro.
        let (mut name, mut armed) = (0..0, false);
        // The nodes open, with the place each has in the table.
        let mut open: Vec<Open> = Vec::new();
        let mut held: Option<Held> = None;
        let mut expansions: Vec<Expansion> = Vec::new();
        // The library answers whether the walk reached the end of the text, which a whole file's walk always does.
        let _: bool = lexed.intersperse_trivia(&output, &mut |step| {
            let step = step_of(step);
            if let Some(each) = held.as_mut() {
                match &step {
                    Step::Enter(_) => each.depth.0 += 1,
                    Step::Exit => each.depth.0 -= 1,
                    Step::Token(..) | Step::Error(..) => {}
                }
                each.steps.push(step.clone());
                if each.depth.0 == 0
                    && let Some(done) = held.take()
                {
                    open.truncate(open.len().saturating_sub(1));
                    if let Some(arguments) =
                        expanded(&done.name, RustText(whole), done.start, &done.steps)
                    {
                        expansions.push(Expansion {
                            host: done.host,
                            arguments,
                        });
                    }
                }
                emit(builder, RustText(whole), &step, &mut cursor);
                return;
            }
            match &step {
                Step::Token(SyntaxKind::IDENT, length) => {
                    let from = index(cursor.position).0;
                    name = from..from.saturating_add(length.0);
                    armed = false;
                }
                Step::Token(SyntaxKind::BANG, _) => {
                    armed = whole
                        .get(name.clone())
                        .is_some_and(|written| expanding.0.contains(&written));
                }
                Step::Token(SyntaxKind::WHITESPACE | SyntaxKind::COMMENT, _) | Step::Exit => {}
                Step::Enter(SyntaxKind::TOKEN_TREE) if armed => {
                    armed = false;
                    held = Some(Held {
                        name: MacroName(whole.get(name.clone()).unwrap_or_default().into()),
                        host: open.last().map_or(Position(0), |each| each.at),
                        start: cursor.position,
                        depth: Count(1),
                        steps: vec![step.clone()],
                    });
                    open.push(Open { at: cursor.next });
                    emit(builder, RustText(whole), &step, &mut cursor);
                    return;
                }
                Step::Enter(_) | Step::Token(..) | Step::Error(..) => armed = false,
            }
            match &step {
                Step::Enter(_) => open.push(Open { at: cursor.next }),
                Step::Exit => open.truncate(open.len().saturating_sub(1)),
                Step::Token(..) | Step::Error(..) => {}
            }
            emit(builder, RustText(whole), &step, &mut cursor);
        });
        // The arguments are added once the file's own tree is whole, each under its call, spanning the text they were read from.
        for (number, expansion) in expansions.iter().enumerate() {
            languages::adopt(builder, expansion.host);
            let made = languages::ExpansionId(u32::try_from(number).unwrap_or(u32::MAX));
            for argument in &expansion.arguments {
                cursor.position = argument.from;
                for step in &argument.steps {
                    if matches!(step, Step::Enter(_) | Step::Token(..)) {
                        languages::mark_expansion(builder, cursor.next, expansion.host, made);
                    }
                    emit(builder, RustText(whole), step, &mut cursor);
                }
            }
            languages::release(builder);
        }
    }
}

// The arguments of a call of a standard expression macro, read again as the expressions (or the pattern) they hold.
/// Whether the child is white space or a comment.
pub(super) fn is_trivia(child: &Child) -> Triviality {
    if matches!(child.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT) {
        Triviality::Trivia
    } else {
        Triviality::Significant
    }
}

/// The direct children of the token tree held in `steps`, between its delimiters, with the offsets from `start`; none when the steps
/// are not a clean token tree.
pub(super) fn children_of(steps: &[Step], start: Offset) -> Option<Vec<Child>> {
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

/// The steps of `text` read as `reading`, with no error; nothing when it is not all one of them.
pub(super) fn parsed(text: RustText<'_>, reading: Reading) -> Option<Vec<Step>> {
    let lexed = LexedStr::new(Edition::CURRENT, text.0);
    if lexed.errors().next().is_some() {
        return None;
    }
    let input = lexed.to_input(Edition::CURRENT);
    let entry = match reading {
        Reading::Expression => TopEntryPoint::Expr,
        Reading::Pattern => TopEntryPoint::Pattern,
    };
    let output = entry.parse(&input);
    let mut steps = Vec::new();
    let mut clean = true;
    // The library answers whether the walk reached the end of the text: what is left over is not part of the one expression.
    let whole = lexed.intersperse_trivia(&output, &mut |step| {
        clean &= !matches!(step, StrStep::Error { .. });
        steps.push(step_of(step));
    });
    (clean && whole).then_some(steps)
}

/// The bytes a stretch of children spans, without the white space at its ends.
pub(super) fn trimmed(children: &[Child]) -> Option<ArgumentSpan> {
    let first = children
        .iter()
        .find(|each| is_trivia(each) == Triviality::Significant)?;
    let last = children
        .iter()
        .rev()
        .find(|each| is_trivia(each) == Triviality::Significant)?;
    Some(ArgumentSpan {
        from: first.from,
        to: last.to,
    })
}

/// The stretches of the arguments of the macro `name` that are read again, each with what it is read as.
pub(super) fn stretches(name: &MacroName, children: &[Child]) -> Option<Vec<Stretch>> {
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
                .filter(|each| is_trivia(each) == Triviality::Significant);
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
                        reading: Reading::Pattern,
                    });
                    found.push(Stretch {
                        span: trimmed(argument.get(at + 1..)?)?,
                        reading: Reading::Expression,
                    });
                }
                None => found.push(Stretch {
                    span,
                    reading: Reading::Pattern,
                }),
            }
        } else {
            found.push(Stretch {
                span,
                reading: Reading::Expression,
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
            steps: parsed(
                RustText(text.0.get(index(from).0..index(to).0)?),
                stretch.reading,
            )?,
        });
    }
    Some(out)
}

/// The text index an offset stands for.
pub(super) fn index(offset: Offset) -> Count {
    Count(usize::try_from(offset.0).unwrap_or(usize::MAX))
}

/// The offset a text index stands for.
pub(super) fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}
