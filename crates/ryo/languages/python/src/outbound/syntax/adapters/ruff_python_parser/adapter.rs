//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::{INVALID_UTF8, NOT_UTF8, PARSER_ERROR, UNSUPPORTED_SYNTAX};
use super::declaration::semantic_syntax_errors;
use super::future::module_future_annotations;
use super::lexical::{offset, readable};
use super::options::options;
use super::types::{
    AnnotationCandidate, AnnotationContext, ByteIndex, FutureAnnotations, LexicallyComplete,
    Readable, SemanticSyntaxDiagnostic, Writer,
};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::PythonParserSettings;
use ruff_python_ast::Mod;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_parser::parse_unchecked;
use ruff_text_size::Ranged;
use ryo::languages::{self, DiagnosticMetadata, DiagnosticPhase, Offset, TreeImpact};
use std::borrow::Cow;

/// The `ruff_python_parser` syntax adapter.
#[derive(Debug, Default)]
pub(crate) struct SyntaxRuffPythonParserAdapter {
    /// The settings the factory injects: the target version the parser reads under.
    pub(crate) settings: PythonParserSettings,
    /// Actual new carrier parse attempts, observed per adapter only in tests.
    #[cfg(test)]
    pub(crate) expansion_parses: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Actual whole-file parses, by `read` and by a semantic session, observed per adapter only in tests.
    #[cfg(test)]
    pub(crate) file_parses: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl SyntaxPort for SyntaxRuffPythonParserAdapter {
    fn begin_semantics(
        &self,
        tree: &languages::Elements,
        source_kind: PythonSourceKind,
        retained: languages::SemanticSlot,
    ) -> languages::SemanticSlot {
        // The tree `read` parsed, when it kept one for this file: the session renders from it and parses nothing.
        let parsed = match retained {
            languages::SemanticSlot::Retained(kept) => {
                match kept.downcast::<ruff_python_parser::Parsed<Mod>>() {
                    Ok(parsed) => Some(*parsed),
                    // Kept by another language: this session parses the file itself.
                    Err(_) => None,
                }
            }
            languages::SemanticSlot::Empty | languages::SemanticSlot::Begun(_) => None,
        };
        languages::SemanticSlot::Begun(Box::new(super::semantic::PythonSemanticSession::new(
            tree,
            source_kind,
            self.settings,
            parsed,
            #[cfg(test)]
            std::sync::Arc::clone(&self.file_parses),
        )))
    }

    fn expand_annotations(
        &self,
        source: SourceText<'_>,
        carriers: &[crate::outbound::syntax::types::DeferredAnnotation],
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let Ok(source) = std::str::from_utf8(source.0) else {
            return;
        };
        for carrier in carriers {
            let (Ok(start), Ok(end)) = (
                usize::try_from(carrier.start.0),
                usize::try_from(carrier.end.0),
            ) else {
                continue;
            };
            let Some(text) = source.get(start..end) else {
                continue;
            };
            #[cfg(test)]
            let _previous = self
                .expansion_parses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let Ok(parsed) = ruff_python_parser::parse_expression(text) else {
                continue;
            };
            let ruff_python_ast::Expr::StringLiteral(literal) = parsed.expr() else {
                continue;
            };
            append_literal(
                builder,
                carrier.host,
                literal,
                languages::NameText(text),
                languages::EmbeddedMapping::Exact(carrier.start),
            );
        }
    }

    fn read(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let Readable(source) = readable(text);
        #[cfg(test)]
        let _previous = self
            .file_parses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let parsed = parse_unchecked(&source, options(source_kind, &self.settings));
        languages::set_syntax_acceptance(
            builder,
            if matches!(&source, Cow::Borrowed(_)) && parsed.errors().is_empty() {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
        let lexical_complete = LexicallyComplete(
            matches!(&source, Cow::Borrowed(_))
                && super::lexical::complete(&parsed, languages::ByteSlice(text.0)).0,
        );
        let length = offset(ByteIndex(text.0.len()));
        let mut future_errors = Vec::new();
        let future_annotations = if parsed.errors().is_empty()
            && let Mod::Module(module) = parsed.syntax()
        {
            module_future_annotations(module, &mut future_errors)
        } else {
            FutureAnnotations(false)
        };
        let mut writer = Writer::new(
            builder,
            parsed.tokens(),
            languages::NameText(&source),
            Offset(0),
            length,
            AnnotationContext::Runtime,
        );
        writer.future_annotations = future_annotations;
        writer.visit_mod(parsed.syntax());
        let mut semantic_errors = if parsed.errors().is_empty() {
            semantic_syntax_errors(
                parsed.syntax(),
                options(source_kind, &self.settings).target_version(),
                future_annotations,
            )
        } else {
            Vec::new()
        };
        if source.len() == text.0.len() && matches!(source, Cow::Owned(_)) {
            diagnose_invalid_utf8(writer.tree.builder, languages::ByteSlice(text.0));
        }
        diagnose_parser_errors(writer.tree.builder, &parsed, lexical_complete);
        semantic_errors.extend(future_errors);
        diagnose_semantic_errors(writer.tree.builder, semantic_errors);
        let refused = std::mem::take(&mut writer.refused);
        for refusal in refused {
            let metadata = if lexical_complete.0
                && refusal.metadata.impact == TreeImpact::Incomplete
                && refusal.metadata.phase == DiagnosticPhase::Parse
            {
                DiagnosticMetadata {
                    phase: refusal.metadata.phase,
                    impact: TreeImpact::LexicallyComplete,
                    code: refusal.metadata.code,
                }
            } else {
                refusal.metadata
            };
            languages::diagnose_with_metadata(
                writer.tree.builder,
                metadata,
                languages::ReportText(&refusal.said.0),
                Offset(refusal.at.start().to_u32()),
                Offset(refusal.at.end().to_u32()),
            );
        }
        let annotations = std::mem::take(&mut writer.annotations);
        for candidate in annotations {
            append_annotation(
                writer.tree.builder,
                &candidate,
                languages::NameText(&source),
                languages::EmbeddedMapping::Exact(Offset(0)),
            );
        }
        // The session of this file, if one is begun, renders from this tree: the file is parsed once.
        languages::retain_for_semantics(builder, Box::new(parsed));
    }
}

/// Write native parser and target-version diagnostics without changing their ordering.
fn diagnose_parser_errors(
    builder: &mut languages::TableBuilder<'_>,
    parsed: &ruff_python_parser::Parsed<Mod>,
    lexical_complete: LexicallyComplete,
) {
    for error in parsed.errors() {
        // The error's own message: the library's `Display` of the whole error adds the byte range the row already holds.
        languages::diagnose_with_metadata(
            builder,
            DiagnosticMetadata {
                phase: DiagnosticPhase::Parse,
                impact: if lexical_complete.0 {
                    TreeImpact::LexicallyComplete
                } else {
                    TreeImpact::Incomplete
                },
                code: PARSER_ERROR,
            },
            languages::ReportText(&error.error.to_string()),
            Offset(error.location.start().to_u32()),
            Offset(error.location.end().to_u32()),
        );
    }
    for error in parsed.unsupported_syntax_errors() {
        languages::diagnose_with_metadata(
            builder,
            DiagnosticMetadata {
                phase: DiagnosticPhase::Validation,
                impact: TreeImpact::Preserved,
                code: UNSUPPORTED_SYNTAX,
            },
            languages::ReportText(&error.to_string()),
            Offset(error.range.start().to_u32()),
            Offset(error.range.end().to_u32()),
        );
    }
}

/// Preserve validation diagnostics without discarding accepted AST facts.
fn diagnose_semantic_errors(
    builder: &mut languages::TableBuilder<'_>,
    errors: Vec<SemanticSyntaxDiagnostic>,
) {
    for error in errors {
        languages::diagnose_with_metadata(
            builder,
            DiagnosticMetadata {
                phase: error.phase,
                impact: error.impact,
                code: error.code,
            },
            languages::ReportText(&error.message.0),
            Offset(error.range.start().to_u32()),
            Offset(error.range.end().to_u32()),
        );
    }
}

/// Report each invalid original byte interval after the parser received replacement text.
fn diagnose_invalid_utf8(
    builder: &mut languages::TableBuilder<'_>,
    source: languages::ByteSlice<'_>,
) {
    let mut at = 0_usize;
    for chunk in source.0.utf8_chunks() {
        at += chunk.valid().len();
        let bad = chunk.invalid().len();
        if bad > 0 {
            languages::diagnose_with_metadata(
                builder,
                DiagnosticMetadata {
                    phase: DiagnosticPhase::Parse,
                    impact: TreeImpact::Incomplete,
                    code: INVALID_UTF8,
                },
                languages::ReportText(NOT_UTF8),
                offset(ByteIndex(at)),
                offset(ByteIndex(at + bad)),
            );
        }
        at += bad;
    }
}

/// Retain a parsed annotation's logical tokens independently of its physical source map.
/// Simple annotations use the parser's original content coordinates. Complex strings
/// are parsed again before relocation so semantic token text and child order survive.
fn append_annotation(
    builder: &mut languages::TableBuilder<'_>,
    candidate: &AnnotationCandidate<'_>,
    source: languages::NameText<'_>,
    inherited: languages::EmbeddedMapping,
) {
    append_literal(
        builder,
        candidate.host,
        candidate.literal,
        source,
        inherited,
    );
}

/// Borrow a resolved carrier's parsed literal; the deferred request never retains or clones its AST.
fn append_literal(
    builder: &mut languages::TableBuilder<'_>,
    host: languages::Position,
    literal: &ruff_python_ast::ExprStringLiteral,
    source: languages::NameText<'_>,
    inherited: languages::EmbeddedMapping,
) {
    let Ok(annotation) = ruff_python_parser::typing::parse_type_annotation(literal, source.0)
    else {
        return;
    };
    let decoded;
    let (parsed, source, start, length, mapping) = if annotation.kind().is_simple() {
        let expression = annotation.expression();
        (
            annotation.parsed(),
            source,
            Offset(expression.start().to_u32()),
            Offset(expression.end().to_u32()),
            inherited,
        )
    } else {
        let logical = literal.value.to_str();
        let Ok(value) = ruff_python_parser::parse_expression(logical) else {
            return;
        };
        decoded = value;
        (
            &decoded,
            languages::NameText(logical),
            Offset(0),
            offset(ByteIndex(logical.len())),
            languages::EmbeddedMapping::WholeHost,
        )
    };
    let mut nested = Vec::new();
    languages::write_embedded(
        builder,
        host,
        languages::ByteSlice(source.0.as_bytes()),
        mapping,
        |logical| {
            let mut writer = Writer::new(
                logical,
                parsed.tokens(),
                source,
                start,
                length,
                AnnotationContext::Annotation,
            );
            writer.visit_expr(parsed.expr());
            nested = writer.annotations;
        },
    );
    for nested_candidate in nested {
        append_annotation(builder, &nested_candidate, source, mapping);
    }
}
