//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::NOT_UTF8;
use super::schemas::{offset, readable};
use super::types::{AnnotationCandidate, AnnotationContext, ByteIndex, Readable, Writer};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_python_ast::{Arguments, Expr, Mod, Parameters, Stmt, TypeParams};
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseOptions, parse_unchecked};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, Offset};
use std::borrow::Cow;
use std::collections::HashSet;

/// The `ruff_python_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRuffPythonParserAdapter;

/// The parser options derived from Python-owned settings and the source path's kind.
pub(super) fn options(
    source_kind: PythonSourceKind,
    settings: &PythonParserSettings,
) -> ParseOptions {
    let source_type = source_type(source_kind);
    let target_version = match settings.target_version() {
        PythonTargetVersion::Py37 => PythonVersion::PY37,
        PythonTargetVersion::Py38 => PythonVersion::PY38,
        PythonTargetVersion::Py39 => PythonVersion::PY39,
        PythonTargetVersion::Py310 => PythonVersion::PY310,
        PythonTargetVersion::Py311 => PythonVersion::PY311,
        PythonTargetVersion::Py312 => PythonVersion::PY312,
        PythonTargetVersion::Py313 => PythonVersion::PY313,
        PythonTargetVersion::Py314 => PythonVersion::PY314,
        PythonTargetVersion::Py315 => PythonVersion::PY315,
        PythonTargetVersion::Latest => PythonVersion::latest(),
    };
    ParseOptions::from(source_type).with_target_version(target_version)
}

pub(super) const fn source_type(source_kind: PythonSourceKind) -> PySourceType {
    match source_kind {
        PythonSourceKind::Python => PySourceType::Python,
        PythonSourceKind::Stub => PySourceType::Stub,
    }
}

struct SemanticSyntaxDiagnostic {
    message: String,
    range: ruff_text_size::TextRange,
}

/// Report local AST semantic errors Ruff classifies as invalid syntax.
fn semantic_syntax_errors(module: &Mod) -> Vec<SemanticSyntaxDiagnostic> {
    let Mod::Module(module) = module else {
        return Vec::new();
    };

    #[derive(Default)]
    struct VisitorState {
        lexical_scope_depth: usize,
        errors: Vec<SemanticSyntaxDiagnostic>,
    }

    impl VisitorState {
        fn report(&mut self, message: impl Into<String>, range: ruff_text_size::TextRange) {
            self.errors.push(SemanticSyntaxDiagnostic {
                message: message.into(),
                range,
            });
        }

        fn duplicate_parameters(&mut self, parameters: &Parameters) {
            if parameters.len() < 2 {
                return;
            }
            let mut names = HashSet::new();
            for parameter in parameters.iter() {
                let name = parameter.name().as_str();
                if !names.insert(name) {
                    self.report(
                        format!(r#"Duplicate parameter "{name}""#),
                        parameter.name().range(),
                    );
                }
            }
        }

        fn type_parameters(&mut self, parameters: &TypeParams) {
            if parameters.len() < 2 {
                return;
            }
            let mut names = HashSet::new();
            for parameter in parameters {
                let name = parameter.name().id.as_str();
                if !names.insert(name) {
                    self.report("duplicate type parameter", parameter.range());
                }
            }
            let mut saw_default = false;
            for parameter in parameters {
                if saw_default && parameter.default().is_none() {
                    self.report(
                        format!(
                            "non default type parameter `{}` follows default type parameter",
                            parameter.name().id.as_str()
                        ),
                        parameter.range(),
                    );
                }
                saw_default |= parameter.default().is_some();
            }
        }

        fn duplicate_keywords(&mut self, arguments: &Arguments) {
            if arguments.keywords.len() < 2 {
                return;
            }
            let mut names = HashSet::new();
            for (name, range) in arguments
                .keywords
                .iter()
                .filter_map(|keyword| keyword.arg.as_ref().map(|name| (name, keyword.range)))
            {
                if !names.insert(name.as_str()) {
                    self.report(
                        format!("Duplicate keyword argument `{}`", name.as_str()),
                        range,
                    );
                }
            }
        }
    }

    impl<'a> Visitor<'a> for VisitorState {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            match stmt {
                Stmt::Nonlocal(nonlocal) if self.lexical_scope_depth == 0 => self.report(
                    "nonlocal declaration not allowed at module level",
                    nonlocal.range,
                ),
                Stmt::FunctionDef(function) => {
                    if let Some(parameters) = &function.type_params {
                        self.type_parameters(parameters);
                    }
                    self.duplicate_parameters(&function.parameters);
                    self.lexical_scope_depth += 1;
                    walk_stmt(self, stmt);
                    self.lexical_scope_depth -= 1;
                }
                Stmt::ClassDef(class) => {
                    if let Some(parameters) = &class.type_params {
                        self.type_parameters(parameters);
                    }
                    if let Some(arguments) = &class.arguments {
                        self.duplicate_keywords(arguments);
                    }
                    self.lexical_scope_depth += 1;
                    walk_stmt(self, stmt);
                    self.lexical_scope_depth -= 1;
                }
                Stmt::TypeAlias(alias) => {
                    if let Some(parameters) = &alias.type_params {
                        self.type_parameters(parameters);
                    }
                    walk_stmt(self, stmt);
                }
                _ => walk_stmt(self, stmt),
            }
        }

        fn visit_expr(&mut self, expr: &'a Expr) {
            match expr {
                Expr::Lambda(lambda) => {
                    if let Some(parameters) = &lambda.parameters {
                        self.duplicate_parameters(parameters);
                    }
                }
                Expr::Call(call) => self.duplicate_keywords(&call.arguments),
                _ => {}
            }
            ruff_python_ast::visitor::walk_expr(self, expr);
        }
    }

    let mut visitor = VisitorState::default();
    for statement in &module.body {
        visitor.visit_stmt(statement);
    }
    visitor.errors
}

impl SyntaxPort for SyntaxRuffPythonParserAdapter {
    fn read(
        &self,
        text: SourceText<'_>,
        source_kind: PythonSourceKind,
        settings: &PythonParserSettings,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let Readable(source) = readable(text);
        let parsed = parse_unchecked(&source, options(source_kind, settings));
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
        let semantic_errors = semantic_syntax_errors(parsed.syntax());
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
        for error in semantic_errors {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&error.message),
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
