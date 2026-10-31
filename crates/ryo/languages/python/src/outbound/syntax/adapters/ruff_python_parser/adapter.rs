//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::NOT_UTF8;
use super::schemas::{offset, readable};
use super::types::{AnnotationCandidate, AnnotationContext, ByteIndex, Readable, Writer};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ruff_python_ast::PythonVersion;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, Offset};
use std::borrow::Cow;

/// The `ruff_python_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRuffPythonParserAdapter;

/// The options the library parses with: a module, in the latest Python it reads.
pub(super) fn options() -> ParseOptions {
    ParseOptions::from(Mode::Module).with_target_version(PythonVersion::latest())
}

impl SyntaxPort for SyntaxRuffPythonParserAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let Readable(source) = readable(text);
        let parsed = parse_unchecked(&source, options());
        let length = offset(ByteIndex(text.0.len()));
        let mut writer = Writer::new(
            builder,
            parsed.tokens(),
            languages::NameText(&source),
            Offset(0),
            length,
            AnnotationContext::Runtime,
        );
        writer.visit_mod(parsed.syntax());
        if source.len() == text.0.len() && matches!(source, Cow::Owned(_)) {
            let mut at = 0_usize;
            for chunk in text.0.utf8_chunks() {
                at += chunk.valid().len();
                let bad = chunk.invalid().len();
                if bad > 0 {
                    languages::diagnose(
                        writer.tree.builder,
                        languages::ReportText(NOT_UTF8),
                        offset(ByteIndex(at)),
                        offset(ByteIndex(at + bad)),
                    );
                }
                at += bad;
            }
        }
        for error in parsed.errors() {
            // The error's own message: the library's `Display` of the whole error adds the byte range the row already holds.
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&error.error.to_string()),
                Offset(error.location.start().to_u32()),
                Offset(error.location.end().to_u32()),
            );
        }
        for error in parsed.unsupported_syntax_errors() {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&error.to_string()),
                Offset(error.range.start().to_u32()),
                Offset(error.range.end().to_u32()),
            );
        }
        let refused = std::mem::take(&mut writer.refused);
        for refusal in refused {
            languages::diagnose(
                writer.tree.builder,
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
    }
}

/// Retain a parsed annotation's logical tokens independently of its physical source map.
/// Simple annotations use the parser's original content coordinates. Complex strings
/// are parsed again before relocation so semantic token text and child order survive.
fn append_annotation(
    builder: &mut languages::TableBuilder<'_>,
    candidate: &AnnotationCandidate,
    source: languages::NameText<'_>,
    inherited: languages::EmbeddedMapping,
) {
    let Ok(annotation) =
        ruff_python_parser::typing::parse_type_annotation(&candidate.literal, source.0)
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
        let logical = candidate.literal.value.to_str();
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
        candidate.host,
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
