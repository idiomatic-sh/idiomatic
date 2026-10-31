//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::NOT_UTF8;
use super::schemas::{offset, readable};
use super::types::{AnnotationContext, ByteIndex, Readable, Writer};
use super::writer::append_annotation;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ruff_python_ast::PythonVersion;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
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
