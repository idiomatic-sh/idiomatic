//! Parse native source, report frontend diagnostics, and schedule embedded expression writing.
use super::constants::{
    ANNOTATED_GLOBAL, BARE_STARRED_TARGET, COMPREHENSION_REBIND, DUPLICATE_KEYWORD_ARGUMENT,
    DUPLICATE_PARAMETER, DUPLICATE_TYPE_PARAMETER, GLOBAL_PARAMETER, INVALID_UTF8,
    NONLOCAL_AT_MODULE, NONLOCAL_PARAMETER, NOT_UTF8, PARSER_ERROR, PATTERN_BINDINGS_MISMATCH,
    TYPE_PARAMETER_DEFAULT_ORDER, UNSUPPORTED_SYNTAX,
};
use super::schemas::{offset, readable};
use super::types::{AnnotationCandidate, AnnotationContext, ByteIndex, Readable, Writer};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
use ruff_python_ast::{Arguments, Expr, Mod, Parameters, Pattern, Stmt, TypeParams};
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseOptions, parse_unchecked};
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
    /// The parser or validation phase that owns this diagnostic.
    phase: DiagnosticPhase,
    /// Whether the diagnostic preserves usable syntax facts.
    impact: TreeImpact,
}

/// Report Python syntax constraints that preserve a complete parsed AST.
#[expect(
    clippy::too_many_lines,
    reason = "one visitor owns nested lexical scopes while checking parser-level declaration constraints"
)]
fn semantic_syntax_errors(
    module: &Mod,
    target_version: PythonVersion,
    future_annotations: bool,
) -> Vec<SemanticSyntaxDiagnostic> {
    /// Compiler pattern validation compares names, independent of their order or nesting.
    fn pattern_bindings<'a>(pattern: &'a Pattern) -> HashSet<&'a str> {
        let mut names = HashSet::new();
        match pattern {
            Pattern::MatchValue(_) | Pattern::MatchSingleton(_) => {}
            Pattern::MatchSequence(sequence) => {
                for child in &sequence.patterns {
                    names.extend(pattern_bindings(child));
                }
            }
            Pattern::MatchMapping(mapping) => {
                for child in &mapping.patterns {
                    names.extend(pattern_bindings(child));
                }
                if let Some(rest) = &mapping.rest {
                    let _new = names.insert(rest.as_str());
                }
            }
            Pattern::MatchClass(class) => {
                for child in &class.arguments.patterns {
                    names.extend(pattern_bindings(child));
                }
                for keyword in &class.arguments.keywords {
                    names.extend(pattern_bindings(&keyword.pattern));
                }
            }
            Pattern::MatchStar(star) => {
                if let Some(name) = &star.name {
                    let _new = names.insert(name.as_str());
                }
            }
            Pattern::MatchAs(capture) => {
                if let Some(child) = &capture.pattern {
                    names.extend(pattern_bindings(child));
                }
                if let Some(name) = &capture.name {
                    let _new = names.insert(name.as_str());
                }
            }
            Pattern::MatchOr(alternatives) => {
                if let Some(first) = alternatives.patterns.first() {
                    names.extend(pattern_bindings(first));
                }
            }
        }
        names
    }

    #[derive(Default)]
    struct VisitorState {
        scopes: Vec<ScopeState>,
        comprehensions: Vec<HashSet<String>>,
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
                phase: DiagnosticPhase::Validation,
                impact: TreeImpact::Preserved,
            });
        }

        /// A bare starred target needs an unpacking tuple or list around it.
        fn bare_starred_target(&mut self, target: &Expr) {
            if let Expr::Starred(starred) = target {
                self.report(
                    BARE_STARRED_TARGET,
                    "starred assignment target must be in a list or tuple",
                    starred.range(),
                );
            }
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
        fn visit_pattern(&mut self, pattern: &'a Pattern) {
            if let Pattern::MatchOr(alternatives) = pattern
                && let Some(first) = alternatives.patterns.first()
            {
                let expected = pattern_bindings(first);
                if alternatives
                    .patterns
                    .iter()
                    .skip(1)
                    .any(|branch| pattern_bindings(branch) != expected)
                {
                    self.report(
                        PATTERN_BINDINGS_MISMATCH,
                        "alternative patterns bind different names",
                        alternatives.range(),
                    );
                }
            }
            walk_pattern(self, pattern);
        }

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
                Stmt::Assign(assignment) => {
                    for target in &assignment.targets {
                        self.bare_starred_target(target);
                    }
                    walk_stmt(self, stmt);
                }
                Stmt::For(for_stmt) => {
                    self.bare_starred_target(&for_stmt.target);
                    walk_stmt(self, stmt);
                }
                Stmt::With(with_stmt) => {
                    for item in &with_stmt.items {
                        if let Some(target) = &item.optional_vars {
                            self.bare_starred_target(target);
                        }
                    }
                    walk_stmt(self, stmt);
                }
                Stmt::Return(_)
                | Stmt::Delete(_)
                | Stmt::AugAssign(_)
                | Stmt::While(_)
                | Stmt::If(_)
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
            let generators: Option<&[ruff_python_ast::Comprehension]> = expr
                .as_list_comp_expr()
                .map(|value| value.generators.as_ref())
                .or_else(|| {
                    expr.as_set_comp_expr()
                        .map(|value| value.generators.as_ref())
                })
                .or_else(|| {
                    expr.as_dict_comp_expr()
                        .map(|value| value.generators.as_ref())
                })
                .or_else(|| {
                    expr.as_generator_expr()
                        .map(|value| value.generators.as_ref())
                });
            if let Some(generators) = generators {
                let mut targets = ComprehensionTargets::default();
                for generator in generators {
                    self.bare_starred_target(&generator.target);
                    targets.visit_expr(&generator.target);
                }
                self.comprehensions.push(targets.names);
                walk_expr(self, expr);
                let _finished_comprehension = self.comprehensions.pop();
                return;
            }
            match expr {
                Expr::Lambda(lambda) => {
                    if let Some(parameters) = &lambda.parameters {
                        self.duplicate_parameters(parameters);
                        self.visit_parameters(parameters);
                    }
                    // A lambda owns its bindings even when its body occurs inside a comprehension.
                    let outer = std::mem::take(&mut self.comprehensions);
                    self.visit_expr(&lambda.body);
                    self.comprehensions = outer;
                    return;
                }
                Expr::Named(named) => {
                    if let Expr::Name(target) = named.target.as_ref()
                        && self
                            .comprehensions
                            .iter()
                            .any(|names| names.contains(target.id.as_str()))
                    {
                        self.report(
                            COMPREHENSION_REBIND,
                            "assignment expression cannot rebind comprehension variable",
                            target.range(),
                        );
                    }
                }
                Expr::Call(call) => self.duplicate_keywords(&call.arguments),
                Expr::BoolOp(_)
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
            walk_expr(self, expr);
        }
    }

    let Mod::Module(module) = module else {
        return Vec::new();
    };
    let mut visitor = VisitorState {
        scopes: vec![ScopeState::indexed(ScopeKind::Module, None, &module.body)],
        comprehensions: Vec::new(),
        errors: Vec::new(),
    };
    for statement in &module.body {
        visitor.visit_stmt(statement);
    }
    let mut errors = visitor.errors;
    errors.extend(annotation_named_expression_syntax_errors(
        module,
        target_version,
        future_annotations,
    ));
    errors
}

/// Collect the names bound by a comprehension's possibly destructured targets.
#[derive(Default)]
struct ComprehensionTargets {
    /// Names introduced by store-context target expressions.
    names: HashSet<String>,
}

impl<'a> Visitor<'a> for ComprehensionTargets {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr
            && name.ctx == ruff_python_ast::ExprContext::Store
        {
            let _new_target = self.names.insert(name.id.as_str().to_owned());
        }
        walk_expr(self, expr);
    }
}

/// Validate compiler directives and enable features only in a legal module prefix.
/// Relative imports are ordinary imports, and rejected directives never enable a mode.
fn module_future_annotations(
    module: &ruff_python_ast::ModModule,
    errors: &mut Vec<SemanticSyntaxDiagnostic>,
) -> bool {
    struct FutureImports<'a> {
        permitted: &'a HashSet<ruff_text_size::TextRange>,
        enabled: bool,
        valid: bool,
        errors: &'a mut Vec<SemanticSyntaxDiagnostic>,
    }

    impl FutureImports<'_> {
        fn report(&mut self, range: ruff_text_size::TextRange, message: String) {
            self.valid = false;
            self.errors.push(SemanticSyntaxDiagnostic {
                message,
                range,
                // Compiler-directive placement is checked after a complete AST is parsed.
                // Keep the validity row without projecting it as a grammar failure.
                code: super::constants::FUTURE_DIRECTIVE_INVALID,
                phase: DiagnosticPhase::Validation,
                impact: TreeImpact::Preserved,
            });
        }
    }

    impl<'a> Visitor<'a> for FutureImports<'_> {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::ImportFrom(import) = stmt
                && import.level == 0
                && import.module.as_deref() == Some("__future__")
            {
                let valid_position = self.permitted.contains(&import.range());
                if !valid_position {
                    self.report(
                        import.range(),
                        "from __future__ imports must occur at the beginning of the file".into(),
                    );
                }
                if import.is_lazy {
                    self.report(
                        import.range(),
                        "future statements cannot be lazy imports".into(),
                    );
                }
                let known = import
                    .names
                    .iter()
                    .all(|alias| super::constants::FUTURE_FEATURES.contains(&alias.name.as_str()));
                self.valid &= known;
                self.enabled |= valid_position
                    && !import.is_lazy
                    && known
                    && import
                        .names
                        .iter()
                        .any(|alias| alias.name.as_str() == "annotations");
            }
            walk_stmt(self, stmt);
        }
    }

    let mut permitted = HashSet::new();
    for (index, statement) in module.body.iter().enumerate() {
        if index == 0
            && matches!(statement, Stmt::Expr(expression) if expression.value.is_string_literal_expr())
        {
            continue;
        }
        if let Stmt::ImportFrom(import) = statement
            && import.level == 0
            && import.module.as_deref() == Some("__future__")
        {
            let _was_new = permitted.insert(import.range());
        } else {
            break;
        }
    }
    let mut visitor = FutureImports {
        permitted: &permitted,
        enabled: false,
        valid: true,
        errors,
    };
    for statement in &module.body {
        visitor.visit_stmt(statement);
    }
    visitor.enabled && visitor.valid
}

/// Validate expressions inside native deferred or generic annotation scopes.
#[derive(Default)]
struct AnnotationVisitor {
    /// Nesting within an expression whose annotation grammar restricts named expressions.
    annotation_depth: usize,
    /// Whether a valid module-prefix future statement enables postponed annotations.
    future_annotations: bool,
    /// Whether the configured language version uses native deferred annotations.
    deferred_runtime_annotations: bool,
    /// Language validity diagnostics accumulated without discarding the parsed tree.
    errors: Vec<SemanticSyntaxDiagnostic>,
}

impl AnnotationVisitor {
    /// Check an annotation when its language scope imposes the restricted grammar.
    fn annotation(&mut self, expression: &Expr, generic: bool) {
        if !(self.future_annotations || self.deferred_runtime_annotations || generic) {
            return;
        }
        self.invalid_expression(expression);
    }

    /// Enter a restricted expression, then restore the surrounding annotation depth.
    fn invalid_expression(&mut self, expression: &Expr) {
        self.annotation_depth += 1;
        self.visit_expr(expression);
        self.annotation_depth -= 1;
    }
}

impl<'a> Visitor<'a> for AnnotationVisitor {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                for parameter in function.parameters.iter_non_variadic_params() {
                    if let Some(annotation) = parameter.annotation() {
                        self.annotation(annotation, function.type_params.is_some());
                    }
                }
                for parameter in [&function.parameters.vararg, &function.parameters.kwarg]
                    .into_iter()
                    .flatten()
                {
                    if let Some(annotation) = parameter.annotation() {
                        self.annotation(annotation, function.type_params.is_some());
                    }
                }
                if let Some(returns) = function.returns.as_deref() {
                    self.annotation(returns, function.type_params.is_some());
                }
                walk_stmt(self, stmt);
            }
            Stmt::ClassDef(class) => {
                if class.type_params.is_some()
                    && let Some(arguments) = &class.arguments
                {
                    for argument in &arguments.args {
                        self.invalid_expression(argument);
                    }
                    for keyword in &arguments.keywords {
                        self.invalid_expression(&keyword.value);
                    }
                }
                walk_stmt(self, stmt);
            }
            Stmt::AnnAssign(assignment) => {
                if self.future_annotations || self.deferred_runtime_annotations {
                    self.invalid_expression(&assignment.annotation);
                }
                walk_stmt(self, stmt);
            }
            Stmt::TypeAlias(alias) => {
                self.invalid_expression(&alias.value);
                walk_stmt(self, stmt);
            }
            Stmt::ImportFrom(_)
            | Stmt::Return(_)
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
            | Stmt::Global(_)
            | Stmt::Nonlocal(_)
            | Stmt::Expr(_)
            | Stmt::Pass(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::IpyEscapeCommand(_) => walk_stmt(self, stmt),
        }
    }

    fn visit_type_param(&mut self, type_param: &'a ruff_python_ast::TypeParam) {
        if let ruff_python_ast::TypeParam::TypeVar(variable) = type_param
            && let Some(bound) = &variable.bound
        {
            self.invalid_expression(bound);
        }
        if let Some(default) = type_param.default() {
            self.invalid_expression(default);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Lambda(lambda) = expr {
            if let Some(parameters) = &lambda.parameters {
                for default in parameters
                    .iter_non_variadic_params()
                    .filter_map(|parameter| parameter.default.as_deref())
                {
                    self.visit_expr(default);
                }
            }
            // A lambda body introduces a nested scope; its defaults stay in the annotation scope.
            let enclosing_depth = self.annotation_depth;
            self.annotation_depth = 0;
            self.visit_expr(&lambda.body);
            self.annotation_depth = enclosing_depth;
            return;
        }
        if self.annotation_depth > 0
            && let Expr::Named(named) = expr
        {
            self.errors.push(SemanticSyntaxDiagnostic {
                message: "named expression cannot be used within a type annotation".into(),
                range: named.range(),
                code: PARSER_ERROR,
                phase: DiagnosticPhase::Parse,
                // This language validity error preserves the accepted AST and complete
                // token stream; diagnostics and usable syntax facts coexist.
                impact: TreeImpact::Preserved,
            });
        }
        walk_expr(self, expr);
    }
}

/// Emit assignment-expression syntax constraints for deferred and generic annotations.
fn annotation_named_expression_syntax_errors(
    module: &ruff_python_ast::ModModule,
    target_version: PythonVersion,
    future_annotations: bool,
) -> Vec<SemanticSyntaxDiagnostic> {
    let mut visitor = AnnotationVisitor {
        future_annotations,
        deferred_runtime_annotations: target_version > PythonVersion::PY313,
        ..AnnotationVisitor::default()
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
        languages::set_syntax_acceptance(
            builder,
            if parsed.errors().is_empty() {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
        let lexical_complete =
            matches!(&source, Cow::Borrowed(_)) && super::lexical::complete(&parsed, text.0);
        let length = offset(ByteIndex(text.0.len()));
        let mut future_errors = Vec::new();
        let future_annotations = if parsed.errors().is_empty()
            && let Mod::Module(module) = parsed.syntax()
        {
            module_future_annotations(module, &mut future_errors)
        } else {
            false
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
                options(source_kind, settings).target_version(),
                future_annotations,
            )
        } else {
            Vec::new()
        };
        if source.len() == text.0.len() && matches!(source, Cow::Owned(_)) {
            diagnose_invalid_utf8(writer.tree.builder, text.0);
        }
        diagnose_parser_errors(writer.tree.builder, &parsed, lexical_complete);
        semantic_errors.extend(future_errors);
        diagnose_semantic_errors(writer.tree.builder, semantic_errors);
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

/// Write native parser and target-version diagnostics without changing their ordering.
fn diagnose_parser_errors(
    builder: &mut languages::TableBuilder<'_>,
    parsed: &ruff_python_parser::Parsed<Mod>,
    lexical_complete: bool,
) {
    for error in parsed.errors() {
        // The error's own message: the library's `Display` of the whole error adds the byte range the row already holds.
        languages::diagnose_with_metadata(
            builder,
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
            builder,
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
}

/// Preserve validation diagnostics without discarding accepted AST facts.
fn diagnose_semantic_errors(
    builder: &mut languages::TableBuilder<'_>,
    errors: Vec<SemanticSyntaxDiagnostic>,
) {
    for error in errors {
        languages::diagnose_with_metadata(
            builder,
            DiagnosticMetadata::new(error.phase, error.impact, error.code),
            languages::ReportText(&error.message),
            Offset(error.range.start().to_u32()),
            Offset(error.range.end().to_u32()),
        );
    }
}

/// Report each invalid original byte interval after the parser received replacement text.
fn diagnose_invalid_utf8(builder: &mut languages::TableBuilder<'_>, source: &[u8]) {
    let mut at = 0_usize;
    for chunk in source.utf8_chunks() {
        at += chunk.valid().len();
        let bad = chunk.invalid().len();
        if bad > 0 {
            languages::diagnose_with_metadata(
                builder,
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
