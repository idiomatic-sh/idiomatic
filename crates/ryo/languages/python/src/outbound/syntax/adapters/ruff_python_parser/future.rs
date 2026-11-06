//! Compiler directives: `from __future__` imports are valid only in a module prefix, and only known features enable a mode.

use super::types::SemanticSyntaxDiagnostic;
use ruff_python_ast::Stmt;
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_text_size::Ranged as _;
use ryo::languages::{DiagnosticCode, DiagnosticPhase, TreeImpact};
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
