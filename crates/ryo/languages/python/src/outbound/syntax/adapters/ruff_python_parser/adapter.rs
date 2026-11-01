//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::{
    ANNOTATED_GLOBAL, DUPLICATE_KEYWORD_ARGUMENT, DUPLICATE_PARAMETER, DUPLICATE_TYPE_PARAMETER,
    GLOBAL_PARAMETER, INVALID_UTF8, NONLOCAL_AT_MODULE, NONLOCAL_PARAMETER, NOT_UTF8, PARSER_ERROR,
    TYPE_PARAMETER_DEFAULT_ORDER, UNSUPPORTED_SYNTAX,
};
use super::schemas::{offset, readable};
use super::types::{AnnotationCandidate, AnnotationContext, ByteIndex, Readable, Writer};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::token::Token as RuffToken;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_python_ast::{Arguments, Expr, Mod, Parameters, Stmt, TypeParams};
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseErrorType, ParseOptions, parse_unchecked};
use ruff_text_size::{Ranged as _, TextSize};
use ryo::languages::{self, Offset};
use ryo::languages::{DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// The `ruff_python_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRuffPythonParserAdapter;

/// The parser options derived from Python-owned settings and the source path's kind.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the shared syntax adapter carries immutable parser settings by reference"
)]
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

/// Map the language-owned Python source kind to Ruff's parser source type.
pub(super) const fn source_type(source_kind: PythonSourceKind) -> PySourceType {
    match source_kind {
        PythonSourceKind::Python => PySourceType::Python,
        PythonSourceKind::Stub => PySourceType::Stub,
    }
}

/// One parser-library syntax diagnostic, before byte-range normalization.
struct SemanticSyntaxDiagnostic {
    /// The diagnostic text emitted by the parser.
    message: String,
    /// The parser-native source range.
    range: ruff_text_size::TextRange,
    /// Stable provider-owned classification code.
    code: DiagnosticCode,
}

/// Report local AST semantic errors Ruff classifies as invalid syntax.
#[expect(
    clippy::too_many_lines,
    reason = "one visitor owns nested lexical scopes while checking parser-level declaration constraints"
)]
fn semantic_syntax_errors(module: &Mod) -> Vec<SemanticSyntaxDiagnostic> {
    #[derive(Default)]
    struct VisitorState {
        scopes: Vec<ScopeState>,
        errors: Vec<SemanticSyntaxDiagnostic>,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ScopeKind {
        Module,
        Function,
        Class,
    }

    struct ScopeState {
        kind: ScopeKind,
        parameters: HashSet<String>,
        globals: HashMap<String, TextSize>,
    }

    #[derive(Default)]
    struct GlobalDeclarationIndexer {
        globals: HashMap<String, TextSize>,
    }

    impl<'a> Visitor<'a> for GlobalDeclarationIndexer {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            match stmt {
                Stmt::Global(global) => {
                    for name in &global.names {
                        let _previous_global = self
                            .globals
                            .insert(name.as_str().to_owned(), name.range().start());
                    }
                }
                Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
                Stmt::Return(_)
                | Stmt::Delete(_)
                | Stmt::TypeAlias(_)
                | Stmt::Assign(_)
                | Stmt::AugAssign(_)
                | Stmt::AnnAssign(_)
                | Stmt::For(_)
                | Stmt::While(_)
                | Stmt::If(_)
                | Stmt::With(_)
                | Stmt::Match(_)
                | Stmt::Raise(_)
                | Stmt::Try(_)
                | Stmt::Assert(_)
                | Stmt::Import(_)
                | Stmt::ImportFrom(_)
                | Stmt::Nonlocal(_)
                | Stmt::Expr(_)
                | Stmt::Pass(_)
                | Stmt::Break(_)
                | Stmt::Continue(_)
                | Stmt::IpyEscapeCommand(_) => walk_stmt(self, stmt),
            }
        }
    }

    impl ScopeState {
        fn indexed(kind: ScopeKind, parameters: Option<&Parameters>, body: &[Stmt]) -> Self {
            let parameters = parameters.map_or_else(HashSet::new, |parameters| {
                parameters
                    .iter()
                    .map(|parameter| parameter.name().as_str().to_owned())
                    .collect()
            });
            let mut indexer = GlobalDeclarationIndexer::default();
            for statement in body {
                indexer.visit_stmt(statement);
            }
            Self {
                kind,
                parameters,
                globals: indexer.globals,
            }
        }
    }

    impl VisitorState {
        fn scope(&self) -> Option<&ScopeState> {
            self.scopes.last()
        }

        fn report(
            &mut self,
            code: DiagnosticCode,
            message: impl Into<String>,
            range: ruff_text_size::TextRange,
        ) {
            self.errors.push(SemanticSyntaxDiagnostic {
                message: message.into(),
                range,
                code,
            });
        }

        fn duplicate_parameters(&mut self, parameters: &Parameters) {
            if parameters.len() < 2 {
                return;
            }
            let mut names = HashSet::new();
            for parameter in parameters {
                let name = parameter.name().as_str();
                if !names.insert(name) {
                    self.report(
                        DUPLICATE_PARAMETER,
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
                    self.report(
                        DUPLICATE_TYPE_PARAMETER,
                        "duplicate type parameter",
                        parameter.range(),
                    );
                }
            }
            let mut saw_default = false;
            for parameter in parameters {
                if saw_default && parameter.default().is_none() {
                    self.report(
                        TYPE_PARAMETER_DEFAULT_ORDER,
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
                        DUPLICATE_KEYWORD_ARGUMENT,
                        format!("Duplicate keyword argument `{}`", name.as_str()),
                        range,
                    );
                }
            }
        }
    }

    impl<'a> Visitor<'a> for VisitorState {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            debug_assert!(
                self.scope().is_some(),
                "the module scope is installed before AST traversal"
            );
            match stmt {
                Stmt::Nonlocal(nonlocal) => {
                    if self
                        .scope()
                        .is_some_and(|scope| scope.kind == ScopeKind::Module)
                    {
                        self.report(
                            NONLOCAL_AT_MODULE,
                            "nonlocal declaration not allowed at module level",
                            nonlocal.range,
                        );
                    } else {
                        for name in &nonlocal.names {
                            if self
                                .scope()
                                .is_some_and(|scope| scope.parameters.contains(name.as_str()))
                            {
                                self.report(
                                    NONLOCAL_PARAMETER,
                                    format!(
                                        "name `{}` cannot refer to a parameter and a nonlocal variable",
                                        name.as_str()
                                    ),
                                    name.range(),
                                );
                            }
                        }
                    }
                }
                Stmt::Global(global) => {
                    for name in &global.names {
                        if self
                            .scope()
                            .is_some_and(|scope| scope.parameters.contains(name.as_str()))
                        {
                            self.report(
                                GLOBAL_PARAMETER,
                                format!(
                                    "name `{}` cannot refer to a parameter and a global variable",
                                    name.as_str()
                                ),
                                name.range(),
                            );
                        }
                    }
                }
                Stmt::AnnAssign(assignment) => {
                    if let Expr::Name(name) = assignment.target.as_ref() {
                        let global = self
                            .scope()
                            .and_then(|scope| scope.globals.get(name.id.as_str()).copied());
                        if let Some(global) = global {
                            let is_module = self
                                .scope()
                                .is_some_and(|scope| scope.kind == ScopeKind::Module);
                            if !is_module || name.range().start() < global {
                                self.report(
                                    ANNOTATED_GLOBAL,
                                    format!(
                                        "annotated name `{}` can't be global",
                                        name.id.as_str()
                                    ),
                                    name.range(),
                                );
                            }
                        }
                    }
                    walk_stmt(self, stmt);
                }
                Stmt::FunctionDef(function) => {
                    if let Some(parameters) = &function.type_params {
                        self.type_parameters(parameters);
                    }
                    self.duplicate_parameters(&function.parameters);
                    self.scopes.push(ScopeState::indexed(
                        ScopeKind::Function,
                        Some(&function.parameters),
                        &function.body,
                    ));
                    walk_stmt(self, stmt);
                    let _popped_scope = self.scopes.pop();
                }
                Stmt::ClassDef(class) => {
                    if let Some(parameters) = &class.type_params {
                        self.type_parameters(parameters);
                    }
                    if let Some(arguments) = &class.arguments {
                        self.duplicate_keywords(arguments);
                    }
                    self.scopes
                        .push(ScopeState::indexed(ScopeKind::Class, None, &class.body));
                    walk_stmt(self, stmt);
                    let _popped_scope = self.scopes.pop();
                }
                Stmt::TypeAlias(alias) => {
                    if let Some(parameters) = &alias.type_params {
                        self.type_parameters(parameters);
                    }
                    walk_stmt(self, stmt);
                }
                Stmt::Return(_)
                | Stmt::Delete(_)
                | Stmt::Assign(_)
                | Stmt::AugAssign(_)
                | Stmt::For(_)
                | Stmt::While(_)
                | Stmt::If(_)
                | Stmt::With(_)
                | Stmt::Match(_)
                | Stmt::Raise(_)
                | Stmt::Try(_)
                | Stmt::Assert(_)
                | Stmt::Import(_)
                | Stmt::ImportFrom(_)
                | Stmt::Expr(_)
                | Stmt::Pass(_)
                | Stmt::Break(_)
                | Stmt::Continue(_)
                | Stmt::IpyEscapeCommand(_) => walk_stmt(self, stmt),
            }
        }

        fn visit_expr(&mut self, expr: &'a Expr) {
            debug_assert!(
                self.scope().is_some(),
                "expression traversal always has an enclosing module or lexical scope"
            );
            match expr {
                Expr::Lambda(lambda) => {
                    if let Some(parameters) = &lambda.parameters {
                        self.duplicate_parameters(parameters);
                    }
                }
                Expr::Call(call) => self.duplicate_keywords(&call.arguments),
                Expr::BoolOp(_)
                | Expr::Named(_)
                | Expr::BinOp(_)
                | Expr::UnaryOp(_)
                | Expr::If(_)
                | Expr::Dict(_)
                | Expr::Set(_)
                | Expr::ListComp(_)
                | Expr::SetComp(_)
                | Expr::DictComp(_)
                | Expr::Generator(_)
                | Expr::Await(_)
                | Expr::Yield(_)
                | Expr::YieldFrom(_)
                | Expr::Compare(_)
                | Expr::FString(_)
                | Expr::TString(_)
                | Expr::StringLiteral(_)
                | Expr::BytesLiteral(_)
                | Expr::NumberLiteral(_)
                | Expr::BooleanLiteral(_)
                | Expr::NoneLiteral(_)
                | Expr::EllipsisLiteral(_)
                | Expr::Attribute(_)
                | Expr::Subscript(_)
                | Expr::Starred(_)
                | Expr::Name(_)
                | Expr::List(_)
                | Expr::Tuple(_)
                | Expr::Slice(_)
                | Expr::IpyEscapeCommand(_) => {}
            }
            ruff_python_ast::visitor::walk_expr(self, expr);
        }
    }

    let Mod::Module(module) = module else {
        return Vec::new();
    };
    let mut visitor = VisitorState {
        scopes: vec![ScopeState::indexed(ScopeKind::Module, None, &module.body)],
        errors: Vec::new(),
    };
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
        let lexical_complete = matches!(&source, Cow::Borrowed(_))
            && token_stream_is_complete(parsed.tokens(), text.0)
            && !parsed.errors().iter().any(|error| {
                matches!(
                    error.error,
                    ParseErrorType::Lexical(_)
                        | ParseErrorType::FStringError(_)
                        | ParseErrorType::TStringError(_)
                )
            });
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
                    languages::diagnose_with_metadata(
                        writer.tree.builder,
                        DiagnosticMetadata::new(
                            DiagnosticPhase::Parse,
                            TreeImpact::Incomplete,
                            INVALID_UTF8,
                        ),
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
            languages::diagnose_with_metadata(
                writer.tree.builder,
                DiagnosticMetadata::new(
                    DiagnosticPhase::Parse,
                    if lexical_complete {
                        TreeImpact::LexicallyComplete
                    } else {
                        TreeImpact::Incomplete
                    },
                    PARSER_ERROR,
                ),
                languages::ReportText(&error.error.to_string()),
                Offset(error.location.start().to_u32()),
                Offset(error.location.end().to_u32()),
            );
        }
        for error in parsed.unsupported_syntax_errors() {
            languages::diagnose_with_metadata(
                writer.tree.builder,
                DiagnosticMetadata::new(
                    DiagnosticPhase::Validation,
                    TreeImpact::Preserved,
                    UNSUPPORTED_SYNTAX,
                ),
                languages::ReportText(&error.to_string()),
                Offset(error.range.start().to_u32()),
                Offset(error.range.end().to_u32()),
            );
        }
        for error in semantic_errors {
            languages::diagnose_with_metadata(
                writer.tree.builder,
                DiagnosticMetadata::new(
                    DiagnosticPhase::Validation,
                    TreeImpact::Preserved,
                    error.code,
                ),
                languages::ReportText(&error.message),
                Offset(error.range.start().to_u32()),
                Offset(error.range.end().to_u32()),
            );
        }
        let refused = std::mem::take(&mut writer.refused);
        for refusal in refused {
            let metadata = if lexical_complete
                && refusal.metadata.impact == TreeImpact::Incomplete
                && refusal.metadata.phase == DiagnosticPhase::Parse
            {
                DiagnosticMetadata::new(
                    refusal.metadata.phase,
                    TreeImpact::LexicallyComplete,
                    refusal.metadata.code,
                )
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
    }
}

/// Whether the parser returned a complete ordered token stream covering all non-whitespace input through EOF.
fn token_stream_is_complete(tokens: &[RuffToken], source: &[u8]) -> bool {
    if u32::try_from(source.len()).is_err() {
        return false;
    }
    let mut previous_end = 0_u32;
    for (index, token) in tokens.iter().enumerate() {
        let start = token.start().to_u32();
        let end = token.end().to_u32();
        let start_index = usize::try_from(start).unwrap_or(usize::MAX);
        let end_index = usize::try_from(end).unwrap_or(usize::MAX);
        if start < previous_end
            || start > end
            || end_index > source.len()
            || !source
                .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..start_index)
                .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace))
        {
            return false;
        }
        let kind = super::schemas::token_kind(token.kind());
        if kind
            == crate::domain::syntax::types::Kind::Token(
                crate::domain::tokens::types::Token::Errortoken,
            )
        {
            return false;
        }
        if kind
            == crate::domain::syntax::types::Kind::Token(
                crate::domain::tokens::types::Token::Endmarker,
            )
        {
            if index + 1 != tokens.len() || end_index != source.len() {
                return false;
            }
        }
        previous_end = end;
    }
    source
        .get(usize::try_from(previous_end).unwrap_or(usize::MAX)..)
        .is_some_and(|gap| gap.iter().all(u8::is_ascii_whitespace))
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
