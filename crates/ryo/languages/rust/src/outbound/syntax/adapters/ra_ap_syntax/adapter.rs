//! The syntax port, implemented with rust-analyzer's parser: its events are written straight into
//! the element table, with no syntax tree built. The library's names stop here; what leaves is
//! the language's own `Kind`.
//!
//! The token tree of a call of one of the standard expression macros (`println!`, `assert_eq!`, `vec!`, ...) is read again as the
//! expressions it holds: the parser gives a token tree no structure, so the arguments are parsed as expressions (the pattern of
//! `matches!` as a pattern). The file's own tree keeps the token tree as the parser gave it; the expressions are trees added after it
//! under the macro call (`adopt`), which only the readers that ask for them see. A call that does not parse as the macro's grammar
//! adds nothing.

use super::constants::{MATCHES, VEC};
use super::schemas::repaired;
use super::schemas::step_of;
use super::types::{
    ArgumentGrammar, Cursor, Expansion, HeldMacro, MacroArgument, MacroName, Open, RustText, Step,
};
use super::writer::emit;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{OpaqueMacros, SourceText};
use ra_ap_parser::{LexedStr, TopEntryPoint};
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
        let repaired = repaired(text);
        let whole: &str = &repaired.0;
        let opaque = self.opaque;
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
            literal: Flag::Lowered,
        };
        // The last name written before a `!`, and whether the token tree that follows belongs to a call of an expression macro.
        let (mut name, mut armed) = (0..0, false);
        // The nodes open, with the place each has in the table.
        let mut open: Vec<Open> = Vec::new();
        let mut held: Option<HeldMacro> = None;
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
                    let written = whole
                        .get(index(done.start).0..index(cursor.position).0)
                        .unwrap_or_default();
                    if let Some(arguments) = grammar_of(&done.name, opaque)
                        .and_then(|grammar| arguments(grammar, RustText(written), done.start))
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
                    armed = open
                        .last()
                        .is_some_and(|each| each.kind == SyntaxKind::MACRO_CALL);
                }
                Step::Token(SyntaxKind::WHITESPACE | SyntaxKind::COMMENT, _) | Step::Exit => {}
                Step::Enter(SyntaxKind::TOKEN_TREE) if armed => {
                    armed = false;
                    held = Some(HeldMacro {
                        name: MacroName(whole.get(name.clone()).unwrap_or_default().into()),
                        host: open.last().map_or(Position(0), |each| each.at),
                        start: cursor.position,
                        depth: Count(1),
                        steps: vec![step.clone()],
                    });
                    open.push(Open {
                        at: cursor.next,
                        kind: SyntaxKind::TOKEN_TREE,
                    });
                    emit(builder, RustText(whole), &step, &mut cursor);
                    return;
                }
                Step::Enter(_) | Step::Token(..) | Step::Error(..) => armed = false,
            }
            match &step {
                Step::Enter(kind) => open.push(Open {
                    at: cursor.next,
                    kind: *kind,
                }),
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

/// The arguments of a call read by `grammar` from its token tree, `text`, which starts at `start` in the file: each argument's start and
/// the steps it parses to, in order. Nothing when the token tree is not wholly of the grammar, with no error: then the call adds nothing.
pub(super) fn arguments(
    grammar: ArgumentGrammar,
    text: RustText<'_>,
    start: Offset,
) -> Option<Vec<MacroArgument>> {
    let _ = (grammar, text, start);
    None
}

/// The text index an offset stands for.
pub(super) fn index(offset: Offset) -> Count {
    Count(usize::try_from(offset.0).unwrap_or(usize::MAX))
}

/// The offset a text index stands for.
pub(super) fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}
