//! Compiler directives: `from __future__` imports are valid only in a module prefix, and only known features enable a mode.

use super::constants::{
    BRACES, FUTURE, FUTURE_FEATURE_NOT_DEFINED, FUTURE_FEATURES, NOT_A_CHANCE, NOT_DEFINED,
    UNKNOWN_FEATURE,
};
use super::types::{ParserRefusal, Said, SemanticSyntaxDiagnostic};
use ruff_python_ast::Stmt;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_python_ast::{StmtImportFrom, StmtTry};
use ruff_text_size::{Ranged, TextRange};
use ryo::languages::{DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
use std::collections::HashSet;

struct FutureImports<'a> {
    permitted: &'a HashSet<ruff_text_size::TextRange>,
    enabled: bool,
    valid: bool,
    errors: &'a mut Vec<SemanticSyntaxDiagnostic>,
}

impl FutureImports<'_> {
    fn report(&mut self, code: DiagnosticCode, range: ruff_text_size::TextRange, message: String) {
        self.valid = false;
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
            && import.module.as_deref() == Some("__future__")
        {
            let valid_position = self.permitted.contains(&import.range());
            if !valid_position {
                self.report(
                    super::constants::FUTURE_DIRECTIVE_INVALID,
                    import.range(),
                    "from __future__ imports must occur at the beginning of the file".into(),
                );
            }
            if import.is_lazy {
                self.report(
                    super::constants::FUTURE_LAZY_INVALID,
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

/// Validate compiler directives and enable features only in a legal module prefix.
/// Relative imports are ordinary imports, and rejected directives never enable a mode.
pub(super) fn module_future_annotations(
    module: &ruff_python_ast::ModModule,
    errors: &mut Vec<SemanticSyntaxDiagnostic>,
) -> bool {
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
                metadata: DiagnosticMetadata::new(
                    DiagnosticPhase::Validation,
                    TreeImpact::Preserved,
                    FUTURE_FEATURE_NOT_DEFINED,
                ),
            }
        })
}
