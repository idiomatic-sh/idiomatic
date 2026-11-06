//! Compiler declaration validity: the constraints on names, parameters, patterns and scopes that hold for a complete parsed tree.

use super::constants::{
    ANNOTATED_GLOBAL, COMPREHENSION_REBIND, GLOBAL_PARAMETER, NONLOCAL_AT_MODULE,
    NONLOCAL_GLOBAL_CONFLICT, NONLOCAL_PARAMETER, PATTERN_BINDINGS_MISMATCH,
};
use super::scopes::{ScopeKind, ScopeState, VisitorState};
use super::types::SemanticSyntaxDiagnostic;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
use ruff_python_ast::{Expr, Mod, Pattern, PythonVersion, Stmt};
use ruff_text_size::Ranged as _;
use std::collections::HashSet;

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
                        } else if self
                            .scope()
                            .is_some_and(|scope| scope.globals.contains_key(name.as_str()))
                        {
                            self.report(
                                NONLOCAL_GLOBAL_CONFLICT,
                                format!("name `{}` is nonlocal and global", name.as_str()),
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
                                format!("annotated name `{}` can't be global", name.id.as_str()),
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

/// Report Python syntax constraints that preserve a complete parsed AST.
pub(super) fn semantic_syntax_errors(
    module: &Mod,
    target_version: PythonVersion,
    future_annotations: bool,
) -> Vec<SemanticSyntaxDiagnostic> {
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
    errors.extend(
        super::annotation::annotation_named_expression_syntax_errors(
            module,
            target_version,
            future_annotations,
        ),
    );
    errors
}
