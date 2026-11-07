//! The scopes the declaration checks walk: what each scope declares, and the state one walk keeps.

use super::constants::{
    BARE_STARRED_TARGET, BARE_STARRED_TARGET_MESSAGE, DUPLICATE_KEYWORD_ARGUMENT,
    DUPLICATE_PARAMETER, DUPLICATE_TYPE_PARAMETER, DUPLICATE_TYPE_PARAMETER_MESSAGE,
    TYPE_PARAMETER_DEFAULT_ORDER,
};
use super::types::{BoundName, DiagnosticMessage, SemanticSyntaxDiagnostic};
use ruff_python_ast::visitor::{Visitor, walk_stmt};
use ruff_python_ast::{Arguments, Expr, Parameters, Stmt, TypeParams};
use ruff_text_size::Ranged as _;
use ruff_text_size::TextSize;
use ryo::languages::{DiagnosticCode, DiagnosticPhase, TreeImpact};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct VisitorState {
    pub scopes: Vec<ScopeState>,
    pub comprehensions: Vec<HashSet<BoundName>>,
    pub errors: Vec<SemanticSyntaxDiagnostic>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    /// The module scope.
    Module,
    /// A function scope.
    Function,
    /// A class scope.
    Class,
}

pub(super) struct ScopeState {
    pub kind: ScopeKind,
    pub parameters: HashSet<BoundName>,
    pub globals: HashMap<BoundName, TextSize>,
}

#[derive(Default)]
struct GlobalDeclarationIndexer {
    pub globals: HashMap<BoundName, TextSize>,
}

impl<'a> Visitor<'a> for GlobalDeclarationIndexer {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Global(global) => {
                for name in &global.names {
                    let _previous_global = self
                        .globals
                        .insert(BoundName(name.as_str().to_owned()), name.range().start());
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
    pub(super) fn indexed(kind: ScopeKind, parameters: Option<&Parameters>, body: &[Stmt]) -> Self {
        let parameters = parameters.map_or_else(HashSet::new, |parameters| {
            parameters
                .iter()
                .map(|parameter| BoundName(parameter.name().as_str().to_owned()))
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
    pub(super) fn scope(&self) -> Option<&ScopeState> {
        self.scopes.last()
    }

    pub(super) fn report(
        &mut self,
        code: DiagnosticCode,
        message: impl Into<DiagnosticMessage>,
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
    pub(super) fn bare_starred_target(&mut self, target: &Expr) {
        if let Expr::Starred(starred) = target {
            self.report(
                BARE_STARRED_TARGET,
                BARE_STARRED_TARGET_MESSAGE,
                starred.range(),
            );
        }
    }

    pub(super) fn duplicate_parameters(&mut self, parameters: &Parameters) {
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

    pub(super) fn type_parameters(&mut self, parameters: &TypeParams) {
        if parameters.len() < 2 {
            return;
        }
        let mut names = HashSet::new();
        for parameter in parameters {
            let name = parameter.name().id.as_str();
            if !names.insert(name) {
                self.report(
                    DUPLICATE_TYPE_PARAMETER,
                    DUPLICATE_TYPE_PARAMETER_MESSAGE,
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

    pub(super) fn duplicate_keywords(&mut self, arguments: &Arguments) {
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
