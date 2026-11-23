//! Walk native source into the engine's node table: parse it, run the walker over its tree, report what the library and the
//! language said of the file, and lend the walked tree while the parse lives.
use super::constants::{INVALID_UTF8, NOT_UTF8, PARSER_ERROR, UNSUPPORTED_SYNTAX};
use super::declaration::semantic_syntax_errors;
use super::embedded::embedded;
use super::future::module_future_annotations;
use super::lexical::{complete, offset, readable};
use super::options::options;
use super::types::{ByteIndex, Depth, FutureAnnotations, Readable, TokenIndex, Walker};
use crate::domain::model::types::BindingAnnotationTiming;
use crate::domain::node::types::Walked;
use crate::domain::node::types::{WalkedTree, Wholeness};
use crate::outbound::syntax::types::{PythonSourceKind, ReportedSyntax, SourceText};
use crate::settings::PythonParserSettings;
use ruff_python_ast::Mod;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_parser::parse_unchecked;
use ruff_text_size::TextRange;
use ryo::languages::{
    self, DiagnosticMetadata, DiagnosticPhase, NameText, Offset, Position, SyntaxCompleteness,
    TreeImpact, node_table,
};
use std::borrow::Cow;

/// One thing said of the file, recorded in the table.
fn report(
    builder: &mut languages::NodeTableBuilder<'_>,
    metadata: DiagnosticMetadata,
    said: NameText<'_>,
    at: TextRange,
) {
    node_table::diagnose_with_metadata(
        builder,
        Offset(at.start().to_u32()),
        Offset(at.end().to_u32()),
        said,
        metadata,
    );
}

/// Reports each interval of bytes that is not UTF-8, where the parser received replacement text; answers how many there are.
fn report_invalid_utf8(
    builder: &mut languages::NodeTableBuilder<'_>,
    source: languages::ByteSlice<'_>,
) -> languages::Count {
    let mut at = 0_usize;
    let mut found = 0_usize;
    for chunk in source.0.utf8_chunks() {
        at += chunk.valid().len();
        let bad = chunk.invalid().len();
        if bad > 0 {
            found += 1;
            node_table::diagnose_with_metadata(
                builder,
                offset(ByteIndex(at)),
                offset(ByteIndex(at + bad)),
                NameText(NOT_UTF8),
                DiagnosticMetadata {
                    phase: DiagnosticPhase::Parse,
                    impact: TreeImpact::Incomplete,
                    code: INVALID_UTF8,
                },
            );
        }
        at += bad;
    }
    languages::Count(found)
}

/// The impact of a refusal of the parser's: the tokens are whole when the lexer read every one.
const fn parse_impact(lexical: SyntaxCompleteness) -> TreeImpact {
    match lexical {
        SyntaxCompleteness::Complete => TreeImpact::LexicallyComplete,
        SyntaxCompleteness::Incomplete => TreeImpact::Incomplete,
    }
}

/// Walks `text` into `builder`'s table, reports what was said of it in the order the language's reader reports it (bytes that are
/// no text, the library's errors, the language's own validation when `reported` asks for every report, then what `CPython`
/// refuses), embeds the expressions of the strings `read` says are type expressions, and hands `read` the walked tree while the parses
/// live.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the shared syntax adapter carries immutable parser settings by reference"
)]
pub(super) fn walked(
    settings: &PythonParserSettings,
    text: SourceText<'_>,
    source_kind: PythonSourceKind,
    reported: ReportedSyntax,
    builder: &mut languages::NodeTableBuilder<'_>,
    names: crate::domain::model::names::types::WalkNames,
    read: &mut dyn FnMut(Walked<'_, '_, '_>) -> Vec<Position>,
) {
    let Readable(source) = readable(text);
    let parsed = parse_unchecked(&source, options(source_kind, settings));
    let parsed = &parsed;
    let utf8 = matches!(&source, Cow::Borrowed(_));
    node_table::set_syntax_acceptance(
        builder,
        if utf8 && parsed.errors().is_empty() {
            languages::SyntaxAcceptance::Accepted
        } else {
            languages::SyntaxAcceptance::Rejected
        },
    );
    let mut future_errors = Vec::new();
    let future_annotations = if parsed.errors().is_empty()
        && let Mod::Module(module) = parsed.syntax()
    {
        module_future_annotations(module, &mut future_errors)
    } else {
        FutureAnnotations(false)
    };
    let mut walker = Walker {
        builder,
        tokens: parsed.tokens(),
        next: TokenIndex(0),
        written: Offset(0),
        strings: Vec::new(),
        parsed_tokens: parsed.tokens(),
        refused: Vec::new(),
        future_annotations,
        annotations_deferred: super::future::annotations_deferred(future_annotations, source_kind),
        declarations: super::future::annotated_declarations(source_kind),
        length: offset(ByteIndex(text.0.len())),
        nodes: Vec::new(),
        parents: Vec::new(),
        subtree_ends: Vec::new(),
        open: Vec::new(),
        fields: Vec::new(),
        origin: Offset(0),
        host: None,
        annotating: Depth(0),
        literals: Vec::new(),
        imports: Vec::new(),
        relative_froms: Vec::new(),
        names,
        text: if utf8 { &source } else { "" },
    };
    walker.visit_mod(parsed.syntax());
    let Walker {
        builder,
        nodes,
        parents,
        subtree_ends,
        refused,
        literals,
        imports,
        relative_froms,
        names,
        ..
    } = walker;
    let unreadable = if source.len() == text.0.len() && !utf8 {
        report_invalid_utf8(builder, languages::ByteSlice(text.0))
    } else {
        languages::Count(0)
    };
    let refuses = refused
        .iter()
        .any(|refusal| refusal.metadata.impact != TreeImpact::Preserved);
    // Whether the lexer read every token is asked only of a file something was refused in: nothing else reads it.
    let lexical = if !parsed.errors().is_empty() || refuses {
        if utf8 && complete(&parsed, languages::ByteSlice(text.0)).0 {
            SyntaxCompleteness::Complete
        } else {
            SyntaxCompleteness::Incomplete
        }
    } else if unreadable.0 > 0 {
        SyntaxCompleteness::Incomplete
    } else {
        SyntaxCompleteness::Complete
    };
    for error in parsed.errors() {
        // The error's own message: the library's `Display` of the whole error adds the byte range the row already holds.
        report(
            builder,
            DiagnosticMetadata {
                phase: DiagnosticPhase::Parse,
                impact: parse_impact(lexical),
                code: PARSER_ERROR,
            },
            NameText(&error.error.to_string()),
            error.location,
        );
    }
    if reported == ReportedSyntax::Every {
        for error in parsed.unsupported_syntax_errors() {
            report(
                builder,
                DiagnosticMetadata {
                    phase: DiagnosticPhase::Validation,
                    impact: TreeImpact::Preserved,
                    code: UNSUPPORTED_SYNTAX,
                },
                NameText(&error.to_string()),
                error.range,
            );
        }
        let mut semantic = if parsed.errors().is_empty() {
            semantic_syntax_errors(
                parsed.syntax(),
                options(source_kind, settings).target_version(),
                future_annotations,
            )
        } else {
            Vec::new()
        };
        semantic.extend(future_errors);
        for error in semantic {
            report(
                builder,
                DiagnosticMetadata {
                    phase: error.phase,
                    impact: error.impact,
                    code: error.code,
                },
                NameText(&error.message.0),
                error.range,
            );
        }
    }
    for refusal in &refused {
        let whole = refusal.metadata.impact == TreeImpact::Preserved;
        if whole && reported != ReportedSyntax::Every {
            continue;
        }
        let metadata = if refusal.metadata.impact == TreeImpact::Incomplete
            && refusal.metadata.phase == DiagnosticPhase::Parse
        {
            DiagnosticMetadata {
                phase: refusal.metadata.phase,
                impact: parse_impact(lexical),
                code: refusal.metadata.code,
            }
        } else {
            refusal.metadata
        };
        report(builder, metadata, NameText(&refusal.said.0), refusal.at);
    }
    let syntax = if parsed.errors().is_empty() && !refuses && unreadable.0 == 0 {
        SyntaxCompleteness::Complete
    } else {
        SyntaxCompleteness::Incomplete
    };
    let tree = WalkedTree {
        nodes,
        parents,
        ends: subtree_ends,
        annotations: if super::future::annotations_deferred(future_annotations, source_kind) {
            BindingAnnotationTiming::Deferred
        } else {
            BindingAnnotationTiming::Immediate
        },
        declarations: super::future::annotated_declarations(source_kind),
        strings: literals,
        imports,
        relative_froms,
        segments: Vec::new(),
        text: NameText(&source),
        completeness: Wholeness { syntax, lexical },
        names,
    };
    embedded(builder, tree, Vec::new(), read);
}
