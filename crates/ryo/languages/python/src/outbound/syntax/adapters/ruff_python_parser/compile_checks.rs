//! The compiler's checks the parser library states and the declaration visitor does not: a case pattern that makes the
//! remaining ones unreachable, a mapping pattern's duplicate key, a class pattern's repeated attribute, a name bound twice
//! in one pattern, a starred expression where none can stand, and an asynchronous comprehension inside a synchronous one
//! before Python 3.11. The library's checker is driven over the module with the one fact these need from the walk: whether
//! an expression sits inside a synchronous comprehension; every other fact it may ask is answered so that it reports
//! nothing else.

use super::constants::COMPILER_CHECK;
use super::types::{DiagnosticMessage, SemanticSyntaxDiagnostic};
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{Expr, ModModule, PythonVersion, Stmt};
use ruff_python_parser::semantic_errors::{
    LazyImportContext, SemanticSyntaxChecker, SemanticSyntaxContext, SemanticSyntaxError,
    SemanticSyntaxErrorKind,
};
use ruff_text_size::TextRange;
use ryo::languages::{DiagnosticPhase, TreeImpact};
use std::cell::{Cell, RefCell};

/// Whether the declaration visitor leaves `kind` to this check.
const fn kept(kind: &SemanticSyntaxErrorKind) -> bool {
    matches!(
        kind,
        SemanticSyntaxErrorKind::IrrefutableCasePattern(_)
            | SemanticSyntaxErrorKind::DuplicateMatchKey(_)
            | SemanticSyntaxErrorKind::DuplicateMatchClassAttribute(_)
            | SemanticSyntaxErrorKind::MultipleCaseAssignment(_)
            | SemanticSyntaxErrorKind::InvalidStarExpression
            | SemanticSyntaxErrorKind::AsyncComprehensionInSyncComprehension(_)
    )
}

/// What the library's checker is told while it walks: the file, the target, and whether the expression in hand sits inside a
/// synchronous comprehension of its function.
struct Context<'s> {
    source: &'s str,
    target: PythonVersion,
    in_sync_comprehension: Cell<bool>,
    found: RefCell<Vec<SemanticSyntaxError>>,
}

impl SemanticSyntaxContext for Context<'_> {
    fn future_annotations_or_stub(&self) -> bool {
        false
    }
    fn lazy_import_context(&self) -> Option<LazyImportContext> {
        None
    }
    fn python_version(&self) -> PythonVersion {
        self.target
    }
    fn source(&self) -> &str {
        self.source
    }
    fn global(&self, _name: &str) -> Option<TextRange> {
        None
    }
    fn has_nonlocal_binding(&self, _name: &str) -> bool {
        true
    }
    fn in_async_context(&self) -> bool {
        true
    }
    fn in_await_allowed_context(&self) -> bool {
        true
    }
    fn in_yield_allowed_context(&self) -> bool {
        true
    }
    fn in_sync_comprehension(&self) -> bool {
        self.in_sync_comprehension.get()
    }
    fn in_class_body_comprehension(&self) -> bool {
        false
    }
    fn in_module_scope(&self) -> bool {
        false
    }
    fn in_function_scope(&self) -> bool {
        true
    }
    fn in_generator_context(&self) -> bool {
        false
    }
    fn in_notebook(&self) -> bool {
        false
    }
    fn report_semantic_error(&self, error: SemanticSyntaxError) {
        if kept(&error.kind) {
            self.found.borrow_mut().push(error);
        }
    }
    fn in_loop_context(&self) -> bool {
        true
    }
    fn is_bound_parameter(&self, _name: &str) -> bool {
        false
    }
}

/// The walk: the checker sees every statement and expression; a comprehension's element and every clause but its first
/// iterable are inside it (a lambda's body too), and a function or class body is inside no comprehension of its own.
struct Walk<'s> {
    checker: SemanticSyntaxChecker,
    context: Context<'s>,
}

impl Walk<'_> {
    /// Visit `inner` with the comprehension flag set to `inside`, restoring it after.
    fn within(&mut self, inside: bool, inner: impl FnOnce(&mut Self)) {
        let outer = self.context.in_sync_comprehension.replace(inside);
        inner(self);
        self.context.in_sync_comprehension.set(outer);
    }

    /// A comprehension: its first iterable in the enclosing scope, the rest in its own, synchronous unless a clause is
    /// asynchronous.
    fn comprehension(&mut self, elements: &[&Expr], generators: &[ruff_python_ast::Comprehension]) {
        let inside = !generators.iter().any(|generator| generator.is_async);
        if let Some(first) = generators.first() {
            self.visit_expr(&first.iter);
        }
        self.within(inside, |walk| {
            for (index, generator) in generators.iter().enumerate() {
                walk.visit_expr(&generator.target);
                if index > 0 {
                    walk.visit_expr(&generator.iter);
                }
                for condition in &generator.ifs {
                    walk.visit_expr(condition);
                }
            }
            for element in elements {
                walk.visit_expr(element);
            }
        });
    }
}

impl<'a> Visitor<'a> for Walk<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.checker.visit_stmt(stmt, &self.context);
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            self.within(false, |walk| walk_stmt(walk, stmt));
        } else {
            walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.checker.visit_expr(expr, &self.context);
        match expr {
            Expr::ListComp(list) => self.comprehension(&[&list.elt], &list.generators),
            Expr::SetComp(set) => self.comprehension(&[&set.elt], &set.generators),
            Expr::Generator(generator) => {
                self.comprehension(&[&generator.elt], &generator.generators)
            }
            Expr::DictComp(dict) => match &dict.key {
                Some(key) => self.comprehension(&[key, &dict.value], &dict.generators),
                None => self.comprehension(&[&dict.value], &dict.generators),
            },
            _ => walk_expr(self, expr),
        }
    }
}

/// The library's errors of the kinds this check keeps, over a module that parsed whole, in source order; one already
/// among `known` (same range) is not repeated.
pub(super) fn compile_errors(
    module: &ModModule,
    source: &str,
    target: PythonVersion,
    known: &[SemanticSyntaxDiagnostic],
) -> Vec<SemanticSyntaxDiagnostic> {
    let mut walk = Walk {
        checker: SemanticSyntaxChecker::new(),
        context: Context {
            source,
            target,
            in_sync_comprehension: Cell::new(false),
            found: RefCell::new(Vec::new()),
        },
    };
    for statement in &module.body {
        walk.visit_stmt(statement);
    }
    let mut found = walk.context.found.into_inner();
    found.sort_by_key(|error| (error.range.start(), error.range.end()));
    found.dedup_by(|later, earlier| later.range == earlier.range && later.kind == earlier.kind);
    found
        .into_iter()
        .filter(|error| !known.iter().any(|each| each.range == error.range))
        .map(|error| SemanticSyntaxDiagnostic {
            message: DiagnosticMessage(error.to_string()),
            range: error.range,
            code: COMPILER_CHECK,
            phase: DiagnosticPhase::Validation,
            impact: TreeImpact::Preserved,
        })
        .collect()
}
