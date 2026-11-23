//! Compiler directives: `from __future__` imports are valid only in a module prefix, and only known features enable a mode.

use super::constants::{
    ANNOTATIONS, BRACES, FUTURE, FUTURE_FEATURE_NOT_DEFINED, FUTURE_FEATURES,
    FUTURE_IMPORT_NOT_FIRST_MESSAGE, LAZY_FUTURE_IMPORT_MESSAGE, NOT_A_CHANCE, NOT_DEFINED,
    UNKNOWN_FEATURE,
};
use super::types::{
    DiagnosticMessage, FutureAnnotations, FutureValidity, ParserRefusal, Said,
    SemanticSyntaxDiagnostic,
};
use crate::outbound::syntax::types::PythonSourceKind;
use ruff_python_ast::Stmt;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_python_ast::{StmtImportFrom, StmtTry};
use ruff_text_size::{Ranged, TextRange};
use ryo::languages::{DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
use std::collections::HashSet;

struct FutureImports<'a> {
    permitted: &'a HashSet<TextRange>,
    enabled: FutureAnnotations,
    valid: FutureValidity,
    errors: &'a mut Vec<SemanticSyntaxDiagnostic>,
}

impl FutureImports<'_> {
    fn report(&mut self, code: DiagnosticCode, range: TextRange, message: DiagnosticMessage) {
        self.valid = FutureValidity(false);
        self.errors.push(SemanticSyntaxDiagnostic {
            message,
            range,
            // Future-statement validity is checked after a complete AST is parsed.
            // Preserve the complete AST; the edge chooses its diagnostic namespace by cause.
            code,
            phase: DiagnosticPhase::Validation,
            impact: TreeImpact::Preserved,
        });
    }
}

impl<'a> Visitor<'a> for FutureImports<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ImportFrom(import) = stmt
            && import.level == 0
            && import.module.as_deref() == Some(FUTURE)
        {
            let valid_position = self.permitted.contains(&import.range());
            if !valid_position {
                self.report(
                    super::constants::FUTURE_DIRECTIVE_INVALID,
                    import.range(),
                    FUTURE_IMPORT_NOT_FIRST_MESSAGE.into(),
                );
            }
            if import.is_lazy {
                self.report(
                    super::constants::FUTURE_LAZY_INVALID,
                    import.range(),
                    LAZY_FUTURE_IMPORT_MESSAGE.into(),
                );
            }
            let known = import
                .names
                .iter()
                .all(|alias| FUTURE_FEATURES.contains(&alias.name.as_str()));
            self.valid.0 &= known;
            self.enabled.0 |= valid_position
                && !import.is_lazy
                && known
                && import
                    .names
                    .iter()
                    .any(|alias| alias.name.as_str() == ANNOTATIONS);
        }
        walk_stmt(self, stmt);
    }
}

/// Whether a module's annotations are postponed: under a valid future import, or in a stub, which is never run.
pub(super) fn annotations_deferred(future: FutureAnnotations, kind: PythonSourceKind) -> bool {
    future.0 || kind == PythonSourceKind::Stub
}

/// What a declaration `x: T` without a value does to `x` in a source of this kind: a stub, which is never run, declares `x` by
/// the typing specification; a module that runs gives it no value.
pub(super) const fn annotated_declarations(kind: PythonSourceKind) -> crate::domain::model::types::AnnotatedDeclarations {
    match kind {
        PythonSourceKind::Stub => crate::domain::model::types::AnnotatedDeclarations::Declared,
        PythonSourceKind::Python => crate::domain::model::types::AnnotatedDeclarations::Unbound,
    }
}

/// The text of the module property that records [`annotated_declarations`].
pub(super) const fn declarations_text(declarations: crate::domain::model::types::AnnotatedDeclarations) -> ryo::languages::NameText<'static> {
    ryo::languages::NameText(match declarations {
        crate::domain::model::types::AnnotatedDeclarations::Declared => crate::domain::syntax::constants::MODULE_DECLARATIONS_DECLARED,
        crate::domain::model::types::AnnotatedDeclarations::Unbound => crate::domain::syntax::constants::MODULE_DECLARATIONS_UNBOUND,
    })
}

/// The text of the module property that records [`annotations_deferred`].
pub(super) const fn annotation_timing(deferred: bool) -> ryo::languages::NameText<'static> {
    ryo::languages::NameText(if deferred {
        crate::domain::syntax::constants::MODULE_ANNOTATIONS_DEFERRED
    } else {
        crate::domain::syntax::constants::MODULE_ANNOTATIONS_IMMEDIATE
    })
}

/// Validate compiler directives and enable features only in a legal module prefix.
/// Relative imports are ordinary imports, and rejected directives never enable a mode.
pub(super) fn module_future_annotations(
    module: &ruff_python_ast::ModModule,
    errors: &mut Vec<SemanticSyntaxDiagnostic>,
) -> FutureAnnotations {
    let mut permitted = HashSet::new();
    for (index, statement) in module.body.iter().enumerate() {
        if index == 0
            && matches!(statement, Stmt::Expr(expression) if expression.value.is_string_literal_expr())
        {
            continue;
        }
        if let Stmt::ImportFrom(import) = statement
            && import.level == 0
            && import.module.as_deref() == Some(FUTURE)
        {
            let _was_new = permitted.insert(import.range());
        } else {
            break;
        }
    }
    let mut visitor = FutureImports {
        permitted: &permitted,
        enabled: FutureAnnotations(false),
        valid: FutureValidity(true),
        errors,
    };
    for statement in &module.body {
        visitor.visit_stmt(statement);
    }
    FutureAnnotations(visitor.enabled.0 && visitor.valid.0)
}

/// Where `CPython`'s parser refuses a `try` the library reads without an error: an `else` with no `except` before it (`CPython`: "expected
/// 'except' or 'finally' block", at the `else`). The library refuses a `try` with neither `except` nor `finally`, and nothing else
/// of this.
#[must_use]
pub(super) fn else_without_except(statement: &StmtTry, tokens: &Tokens) -> Option<TextRange> {
    if !statement.handlers.is_empty() {
        return None;
    }
    let from = statement.body.last().map_or(statement.start(), Ranged::end);
    let to = statement.orelse.first()?.start();
    tokens
        .in_range(TextRange::new(from, to))
        .iter()
        .find(|token| token.kind() == TokenKind::Else)
        .map(Ranged::range)
}

/// Where `CPython`'s parser refuses a feature of `from __future__ import ...` it does not know, and what it says: each alias whose
/// name is none of `__future__`'s features. The library reads any name there without an error.
pub(super) fn unknown_futures(
    statement: &StmtImportFrom,
) -> impl Iterator<Item = ParserRefusal> + '_ {
    let future = statement.level == 0
        && statement
            .module
            .as_ref()
            .is_some_and(|module| module.as_str() == FUTURE);
    statement
        .names
        .iter()
        .filter(move |alias| future && !FUTURE_FEATURES.contains(&alias.name.as_str()))
        .map(|alias| {
            let said = if alias.name.as_str() == BRACES {
                NOT_A_CHANCE.to_owned()
            } else {
                format!("{UNKNOWN_FEATURE}{}{NOT_DEFINED}", alias.name.as_str())
            };
            ParserRefusal {
                at: alias.name.range(),
                said: Said(said),
                metadata: DiagnosticMetadata {
                    phase: DiagnosticPhase::Validation,
                    impact: TreeImpact::Preserved,
                    code: FUTURE_FEATURE_NOT_DEFINED,
                },
            }
        })
}
