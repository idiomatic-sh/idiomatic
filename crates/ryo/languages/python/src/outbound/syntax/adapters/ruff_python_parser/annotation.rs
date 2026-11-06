//! Annotation scopes: named expressions are restricted in deferred and generic annotations.

use super::constants::{NAMED_EXPRESSION_IN_ANNOTATION_MESSAGE, PARSER_ERROR};
use super::types::SemanticSyntaxDiagnostic;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{Expr, PythonVersion, Stmt};
use ruff_text_size::Ranged as _;
use ryo::languages::{DiagnosticPhase, TreeImpact};

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
                message: NAMED_EXPRESSION_IN_ANNOTATION_MESSAGE.into(),
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
pub(super) fn annotation_named_expression_syntax_errors(
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
