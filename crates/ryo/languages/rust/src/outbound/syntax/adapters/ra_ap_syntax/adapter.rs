//! The syntax port, implemented with rust-analyzer's parser: its events are written straight into
//! the element table, with no syntax tree built. The library's names stop here; what leaves is
//! the language's own `Kind`.
//!
//! The token tree of a macro call is read again as the arguments it holds (`interface.rs` § Macros): the whole of it is parsed as one
//! parenthesized list of expressions, its delimiters replaced, so every offset stays the file's (an array for `vec!`; an expression, a
//! pattern and a guard for `matches!`). The arguments are trees added after the file's own, under the call.

use super::constants::{ARRAY_ROOTS, LIST_ROOTS, MATCHES, VEC};
use super::schemas::repaired;
use super::schemas::step_of;
use super::types::{
    ArgumentGrammar, ArgumentList, Cursor, Expansion, HeldMacro, MacroArgument, MacroName,
    NodeChildren, Open, RustText, Step, Walker,
};
use super::writer::emit;
use crate::outbound::syntax::interface::{Output, SyntaxWalk};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{OpaqueMacros, SourceText};
use ra_ap_parser::{LexedStr, PrefixEntryPoint, TopEntryPoint};
use ra_ap_syntax::{Edition, SyntaxKind};
use ryo::languages::{self, Count, Flag, Offset, Position};
/// The `ra_ap_syntax` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRaApSyntaxAdapter {
    /// The built-in macros whose arguments are never read again.
    opaque: OpaqueMacros<'static>,
}

impl SyntaxRaApSyntaxAdapter {
    /// The adapter, reading again the arguments of every macro call but those of the macros `opaque` names.
    pub(crate) const fn new(opaque: OpaqueMacros<'static>) -> Self {
        Self { opaque }
    }
}

impl SyntaxPort for SyntaxRaApSyntaxAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        self.walk(text, builder);
    }
}

impl SyntaxRaApSyntaxAdapter {
    pub(crate) fn walk(&self, text: SourceText<'_>, builder: &mut impl Output) {
        let repaired = repaired(text);
        let whole = RustText(&repaired.0);
        let lexed = LexedStr::new(Edition::CURRENT, whole.0);
        let input = lexed.to_input(Edition::CURRENT);
        let output = TopEntryPoint::SourceFile.parse(&input);
        for (error, message) in lexed.errors() {
            let range = lexed.text_range(error);
            builder.diagnostic(
                message,
                offset(Count(range.start)),
                offset(Count(range.end)),
            );
        }
        let mut cursor = Cursor {
            position: Offset(0),
            next: Position(0),
            literal: Flag::Lowered,
        };
        let mut walker = Walker {
            opaque: self.opaque,
            name_from: Offset(0),
            name_to: Offset(0),
            armed: Flag::Lowered,
            open: Vec::new(),
            held: None,
            found: Vec::new(),
            buffer: ArgumentList::default(),
        };
        // The library answers whether the walk reached the end of the text, which a whole file's walk always does.
        let _: bool = lexed.intersperse_trivia(&output, &mut |step| {
            feed(&mut walker, builder, whole, step_of(step), &mut cursor);
        });
        // The arguments are added once the file's own tree is whole, each under its call, spanning the text they were read from; the
        // calls found in them are added after, under their own calls, until none is left.
        let mut pending = std::collections::VecDeque::from(std::mem::take(&mut walker.found));
        let mut number = 0_u32;
        while let Some(expansion) = pending.pop_front() {
            builder.adopt(expansion.host);
            let made = languages::ExpansionId(number);
            number = number.saturating_add(1);
            for argument in expansion.arguments {
                cursor.position = argument.from;
                walker.open.clear();
                walker.armed = Flag::Lowered;
                walker.held = None;
                for step in argument.steps {
                    if matches!(step, Step::Enter(_) | Step::Token(..)) {
                        builder.origin(cursor.next, expansion.host, made);
                    }
                    feed(&mut walker, builder, whole, step, &mut cursor);
                }
            }
            builder.release();
            pending.extend(walker.found.drain(..));
        }
    }
}

/// Writes `step` of a tree of `text` into the table at the cursor, and keeps the walk's account of the macro calls in it: a call's
/// token tree is held until it is whole, and then its arguments are read, when the call is one whose arguments are code.
pub(super) fn feed(
    walker: &mut Walker,
    builder: &mut impl Output,
    text: RustText<'_>,
    step: Step,
    cursor: &mut Cursor,
) {
    if let Some(each) = walker.held.as_mut() {
        let from = cursor.position;
        emit(builder, text, &step, cursor);
        // Its delimiters are its first token and the last element directly under it, when that is a token. An error is no element: the
        // library reports one that follows the token tree (a missing `;`) before the token tree's exit.
        match step {
            Step::Token(kind, _) if each.depth.0 == 1 => {
                if from == each.start {
                    each.opening = Some(kind);
                }
                each.closing = Some(kind);
            }
            Step::Token(..) | Step::Error(..) => {}
            Step::Enter(_) => {
                each.depth.0 += 1;
                each.closing = None;
            }
            Step::Exit => each.depth.0 -= 1,
        }
        if each.depth.0 == 0
            && let Some(done) = walker.held.take()
        {
            walker.open.truncate(walker.open.len().saturating_sub(1));
            if let Some(arguments) = called(&done, walker, text, cursor.position)
                && !arguments.is_empty()
            {
                walker.found.push(Expansion {
                    host: done.host,
                    arguments,
                });
            }
        }
        return;
    }
    match &step {
        Step::Token(SyntaxKind::IDENT, length) => {
            walker.name_from = cursor.position;
            walker.name_to = offset(Count(index(cursor.position).0.saturating_add(length.0)));
            walker.armed = Flag::Lowered;
        }
        Step::Token(SyntaxKind::BANG, _) => {
            let call = walker
                .open
                .last()
                .is_some_and(|each| each.kind == SyntaxKind::MACRO_CALL);
            let expands = call;
            walker.armed = if expands { Flag::Raised } else { Flag::Lowered };
        }
        Step::Token(SyntaxKind::WHITESPACE | SyntaxKind::COMMENT, _) | Step::Exit => {}
        Step::Enter(SyntaxKind::TOKEN_TREE) if walker.armed == Flag::Raised => {
            walker.armed = Flag::Lowered;
            let name = text
                .0
                .get(index(walker.name_from).0..index(walker.name_to).0);
            walker.held = Some(HeldMacro {
                name: MacroName(name.unwrap_or_default().into()),
                host: walker.open.last().map_or(Position(0), |each| each.at),
                start: cursor.position,
                depth: Count(1),
                opening: None,
                closing: None,
            });
            walker.open.push(Open {
                at: cursor.next,
                kind: SyntaxKind::TOKEN_TREE,
            });
            emit(builder, text, &step, cursor);
            return;
        }
        Step::Enter(_) | Step::Token(..) | Step::Error(..) => walker.armed = Flag::Lowered,
    }
    match &step {
        Step::Enter(kind) => walker.open.push(Open {
            at: cursor.next,
            kind: *kind,
        }),
        Step::Exit => walker.open.truncate(walker.open.len().saturating_sub(1)),
        Step::Token(..) | Step::Error(..) => {}
    }
    emit(builder, text, &step, cursor);
}

/// The arguments of the call whose token tree `done` held, which ends at `end` of `text`; none when the token tree is not closed by the
/// delimiter that opens it, when the macro's arguments are never code, or when they are not wholly of its grammar.
fn called(
    done: &HeldMacro,
    walker: &mut Walker,
    text: RustText<'_>,
    end: Offset,
) -> Option<Vec<MacroArgument>> {
    let closed = matches!(
        (done.opening?, done.closing?),
        (SyntaxKind::L_PAREN, SyntaxKind::R_PAREN)
            | (SyntaxKind::L_BRACK, SyntaxKind::R_BRACK)
            | (SyntaxKind::L_CURLY, SyntaxKind::R_CURLY)
    );
    if !closed {
        return None;
    }
    let grammar = grammar_of(&done.name, walker.opaque)?;
    let written = text.0.get(index(done.start).0..index(end).0)?;
    arguments(grammar, RustText(written), done.start, &mut walker.buffer)
}

/// The grammar the arguments of a call of the macro `name` are read by; none for a macro `opaque` names, whose arguments are never code.
pub(super) fn grammar_of(name: &MacroName, opaque: OpaqueMacros<'_>) -> Option<ArgumentGrammar> {
    if opaque.0.contains(&&**name) {
        None
    } else if **name == *VEC {
        Some(ArgumentGrammar::Vector)
    } else if **name == *MATCHES {
        Some(ArgumentGrammar::Matches)
    } else {
        Some(ArgumentGrammar::Arguments)
    }
}

/// The arguments of a call read by `grammar` from its token tree, `text`, which starts at `start` in the file and opens and closes with
/// a one-byte delimiter: each argument's start and the steps it parses to, in order. Nothing when the token tree is not wholly of the
/// grammar, with no error: then the call adds nothing. `buffer` holds the text that is parsed.
pub(super) fn arguments(
    grammar: ArgumentGrammar,
    text: RustText<'_>,
    start: Offset,
    buffer: &mut ArgumentList,
) -> Option<Vec<MacroArgument>> {
    let inside = text.0.get(1..text.0.len().checked_sub(1)?)?;
    let (opening, closing, roots) = match grammar {
        ArgumentGrammar::Arguments => ('(', ')', LIST_ROOTS.as_slice()),
        ArgumentGrammar::Vector => ('[', ']', ARRAY_ROOTS.as_slice()),
        ArgumentGrammar::Matches => {
            return matched(RustText(inside), offset(Count(index(start).0 + 1)));
        }
    };
    // The delimiters are replaced by those of a tuple (or an array), which keeps every offset the file's.
    buffer.0.clear();
    buffer.0.push(opening);
    buffer.0.push_str(inside);
    buffer.0.push(closing);
    let steps = read_as(RustText(&buffer.0), &TopEntryPoint::Expr)?;
    if !matches!(steps.first(), Some(Step::Enter(root)) if roots.contains(root)) {
        return None;
    }
    let found = children(&steps, start).nodes;
    Some(match grammar {
        ArgumentGrammar::Arguments => found.into_iter().map(named_value).collect(),
        ArgumentGrammar::Vector | ArgumentGrammar::Matches => found,
    })
}

/// The value of a named argument, `width = 3`; any other argument as it is.
fn named_value(argument: MacroArgument) -> MacroArgument {
    if !matches!(
        argument.steps.first(),
        Some(Step::Enter(SyntaxKind::BIN_EXPR))
    ) {
        return argument;
    }
    let NodeChildren { mut nodes, tokens } = children(&argument.steps, argument.from);
    // The label is a path of one name: a path expression, its path, segment and name, and the name's token.
    let named = nodes.first().is_some_and(|label| {
        let mut tokens = label
            .steps
            .iter()
            .filter(|each| matches!(each, Step::Token(..)));
        matches!(
            label.steps.first(),
            Some(Step::Enter(SyntaxKind::PATH_EXPR))
        ) && matches!(tokens.next(), Some(Step::Token(SyntaxKind::IDENT, _)))
            && tokens.next().is_none()
    });
    if named && nodes.len() == 2 && tokens == [SyntaxKind::EQ] {
        nodes.pop().unwrap_or(argument)
    } else {
        argument
    }
}

/// The direct children of the node whose steps are `steps`, which starts at `from`.
fn children(steps: &[Step], from: Offset) -> NodeChildren {
    let mut found = NodeChildren::default();
    let (mut at, mut depth, mut begun, mut begun_at) = (index(from).0, 0_usize, 0_usize, from);
    for (place, step) in steps.iter().enumerate() {
        match step {
            Step::Enter(_) => {
                depth += 1;
                if depth == 2 {
                    (begun, begun_at) = (place, offset(Count(at)));
                }
            }
            Step::Exit => {
                if depth == 2 {
                    found.nodes.push(MacroArgument {
                        from: begun_at,
                        steps: steps.get(begun..=place).unwrap_or_default().to_vec(),
                    });
                }
                depth = depth.saturating_sub(1);
            }
            Step::Token(kind, length) => {
                if depth == 1 && !kind.is_trivia() {
                    found.tokens.push(*kind);
                }
                at += length.0;
            }
            Step::Error(..) => {}
        }
    }
    found
}

/// The arguments of `matches!` in the text between its delimiters, which starts at `start`: the expression, a comma, the pattern and
/// perhaps `if` and a guard, a trailing comma allowed; nothing when the text is not all of them.
fn matched(text: RustText<'_>, start: Offset) -> Option<Vec<MacroArgument>> {
    let lexed = LexedStr::new(Edition::CURRENT, text.0);
    if lexed.errors().next().is_some() {
        return None;
    }
    // How many tokens, white space and comments aside, the expression takes: what is left is the pattern and its guard.
    let output = PrefixEntryPoint::Expr.parse(&lexed.to_input(Edition::CURRENT));
    let taken = output.iter().try_fold(0_usize, |taken, step| match step {
        ra_ap_parser::Step::Token { n_input_tokens, .. } => {
            Some(taken + usize::from(n_input_tokens))
        }
        ra_ap_parser::Step::FloatSplit { .. } => Some(taken + 1),
        ra_ap_parser::Step::Error { .. } => None,
        ra_ap_parser::Step::Enter { .. } | ra_ap_parser::Step::Exit => Some(taken),
    })?;
    let significant: Vec<usize> = (0..lexed.len())
        .filter(|each| !lexed.kind(*each).is_trivia())
        .collect();
    let expression = significant.get(..taken)?;
    if expression.is_empty() || lexed.kind(*significant.get(taken)?) != SyntaxKind::COMMA {
        return None;
    }
    let mut rest = significant.get(taken + 1..)?;
    if let [before @ .., last] = rest
        && lexed.kind(*last) == SyntaxKind::COMMA
    {
        rest = before;
    }
    let (pattern, guard) = match rest
        .iter()
        .position(|each| lexed.kind(*each) == SyntaxKind::IF_KW)
    {
        Some(at) => (rest.get(..at)?, Some(rest.get(at + 1..)?)),
        None => (rest, None),
    };
    let mut pieces = vec![
        (expression, TopEntryPoint::Expr),
        (pattern, TopEntryPoint::Pattern),
    ];
    pieces.extend(guard.map(|guard| (guard, TopEntryPoint::Expr)));
    pieces
        .into_iter()
        .map(|(tokens, entry)| {
            let (from, to) = (
                lexed.text_start(*tokens.first()?),
                lexed.text_range(*tokens.last()?).end,
            );
            Some(MacroArgument {
                from: offset(Count(index(start).0 + from)),
                steps: read_as(RustText(text.0.get(from..to)?), &entry)?,
            })
        })
        .collect()
}

/// The steps of `text` read as `entry`, with no error; nothing when it is not all one of them.
fn read_as(text: RustText<'_>, entry: &TopEntryPoint) -> Option<Vec<Step>> {
    read_fragment(text, entry, |step| matches!(step, Step::Error(..) | Step::Enter(SyntaxKind::ERROR)))
}

pub(super) fn read_fragment(text: RustText<'_>, entry: &TopEntryPoint, invalid: impl Fn(&Step) -> bool) -> Option<Vec<Step>> {
    let lexed = LexedStr::new(Edition::CURRENT, text.0);
    if lexed.errors().next().is_some() {
        return None;
    }
    let output = entry.parse(&lexed.to_input(Edition::CURRENT));
    let mut steps = Vec::new();
    // The library answers whether the walk reached the end of the text: what is left over is not part of the one tree.
    let whole = lexed.intersperse_trivia(&output, &mut |step| steps.push(step_of(step)));
    let clean = !steps
        .iter()
        .any(invalid);
    (clean && whole).then_some(steps)
}

/// The text index an offset stands for.
pub(super) fn index(offset: Offset) -> Count {
    Count(usize::try_from(offset.0).unwrap_or(usize::MAX))
}

/// The offset a text index stands for.
pub(super) fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// Hands each step to a caller's writer behind the port's trait object.
struct Forward<'a>(&'a mut dyn Output);

impl Output for Forward<'_> {
    fn string_values(&self) -> bool { self.0.string_values() }
    fn numeric_values(&self) -> bool { self.0.numeric_values() }
    fn enter(&mut self, kind: languages::KindBits, start: Offset) { self.0.enter(kind, start); }
    fn leave(&mut self, end: Offset) { self.0.leave(end); }
    fn token(&mut self, kind: languages::KindBits, start: Offset, end: Offset, decoded: Option<languages::NameText<'_>>) {
        self.0.token(kind, start, end, decoded);
    }
    fn property(&mut self, key: languages::PropertyId, value: languages::NameText<'_>) { self.0.property(key, value); }
    fn formatted(&mut self, key: languages::PropertyId, value: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.0.formatted(key, value)
    }
    fn diagnostic(&mut self, message: &str, start: Offset, end: Offset) { self.0.diagnostic(message, start, end); }
    fn adopt(&mut self, host: Position) { self.0.adopt(host); }
    fn origin(&mut self, at: Position, host: Position, expansion: languages::ExpansionId) { self.0.origin(at, host, expansion); }
    fn release(&mut self) { self.0.release(); }
}

impl SyntaxWalk for SyntaxRaApSyntaxAdapter {
    fn walk(&self, text: SourceText<'_>, out: &mut dyn Output) {
        SyntaxRaApSyntaxAdapter::walk(self, text, &mut Forward(out));
    }
}
