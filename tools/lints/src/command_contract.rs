//! Command proof obligations derived from compiler-resolved supplied calls.

use clippy_utils::{diagnostics::span_lint, find_binding_init, is_test_function};
use rustc_hir::attrs::AttributeKind;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Attribute, Body, Expr, ExprKind, FnDecl, Node, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, TyCtxt, TypingEnv};
use rustc_span::{Span, sym};

rustc_session::declare_lint! {
    pub COMMAND_SERVICE_CONTRACT,
    Forbid,
    "a command lacks proof of its compiler-resolved supplied service contract"
}

#[derive(Default)]
pub(crate) struct CommandContract {
    commands: Vec<(DefId, Span)>,
    fallible: Vec<DefId>,
    proofs: Vec<(DefId, String)>,
}

rustc_session::impl_lint_pass!(CommandContract => [COMMAND_SERVICE_CONTRACT]);

#[derive(Clone, Copy, PartialEq)]
enum Root {
    Supplied,
    Value,
    Unproven,
}

impl CommandContract {
    pub(crate) fn new(tcx: TyCtxt<'_>) -> Self {
        if tcx
            .sess
            .opts
            .lint_cap
            .is_some_and(|level| !level.is_error())
            || tcx.sess.opts.lint_opts.iter().any(|(name, level)| {
                name.replace('-', "_") == COMMAND_SERVICE_CONTRACT.name_lower()
                    && *level != rustc_session::lint::Level::Forbid
            })
        {
            tcx.sess
                .dcx()
                .err("command service checking requires an unweakened native lint level");
        }
        Self::default()
    }

    // The existing CLI module shape, not a product/command name registry.
    fn command(tcx: TyCtxt<'_>, mut owner: DefId, module: &str) -> Option<DefId> {
        while tcx.def_kind(owner) != DefKind::Mod {
            owner = tcx.opt_parent(owner)?;
        }
        if tcx.item_name(owner).as_str() != module {
            return None;
        }
        let command = tcx.opt_parent(owner)?;
        let resource = tcx.opt_parent(command)?;
        let commands = tcx.opt_parent(resource)?;
        (tcx.def_kind(command) == DefKind::Mod
            && tcx.def_kind(resource) == DefKind::Mod
            && tcx.def_kind(commands) == DefKind::Mod
            && tcx.item_name(commands).as_str() == "commands")
            .then_some(command)
    }

    fn root<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, depth: usize) -> Root {
        if depth > 64 {
            return Root::Unproven;
        }
        match expr.kind {
            ExprKind::AddrOf(_, _, value)
            | ExprKind::Unary(UnOp::Deref, value)
            | ExprKind::Field(value, _) => Self::root(cx, value, depth + 1),
            ExprKind::Path(ref path) => {
                let resolved = cx.qpath_res(path, expr.hir_id);
                let Res::Local(binding) = resolved else {
                    return if resolved
                        .opt_def_id()
                        .is_some_and(|method| Self::interface(cx.tcx, method))
                    {
                        Root::Unproven
                    } else {
                        Root::Value
                    };
                };
                if let Some(value) = find_binding_init(cx, binding) {
                    return Self::root(cx, value, depth + 1);
                }
                for (_, node) in cx.tcx.hir_parent_iter(binding) {
                    match node {
                        Node::Param(_) => return Root::Supplied,
                        Node::LetStmt(local) => {
                            return match local.init {
                                Some(value) if Self::root(cx, value, depth + 1) == Root::Value => {
                                    Root::Value
                                }
                                _ => Root::Unproven,
                            };
                        }
                        Node::Arm(arm) => {
                            // Destructuring a supplied Parts/Result does not create a
                            // new service. A service-returned answer stops at Call.
                            return cx
                                .tcx
                                .hir_parent_iter(arm.hir_id)
                                .find_map(|(_, node)| {
                                    if let Node::Expr(Expr {
                                        kind: ExprKind::Match(value, _, _),
                                        ..
                                    }) = node
                                    {
                                        Some(Self::root(cx, value, depth + 1))
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(Root::Unproven);
                        }
                        Node::Item(_) | Node::Expr(_) => break,
                        _ => {}
                    }
                }
                Root::Unproven
            }
            ExprKind::MethodCall(_, receiver, _, _)
                if cx
                    .typeck_results()
                    .type_dependent_def_id(expr.hir_id)
                    .is_some_and(|method| {
                        cx.tcx
                            .trait_of_assoc(method)
                            .or_else(|| {
                                cx.tcx
                                    .trait_item_of(method)
                                    .and_then(|item| cx.tcx.trait_of_assoc(item))
                            })
                            .is_some_and(|owner| Some(owner) == cx.tcx.lang_items().clone_trait())
                    }) =>
            {
                Self::root(cx, receiver, depth + 1)
            }
            // A returned callable/port has unknown provenance, not the identity of
            // its producer. Ordinary value methods are excluded before this walk.
            ExprKind::Call(..) | ExprKind::MethodCall(..) => Root::Unproven,
            ExprKind::Closure(_) => Root::Value,
            _ => Root::Unproven,
        }
    }

    fn interface(tcx: TyCtxt<'_>, method: DefId) -> bool {
        tcx.trait_of_assoc(method)
            .or_else(|| {
                tcx.trait_item_of(method)
                    .and_then(|item| tcx.trait_of_assoc(item))
            })
            .is_some_and(|owner| {
                crate::boundary::adapters::rustc::adapter::BoundaryRustcAdapter::selected(
                    tcx, owner,
                )
            })
    }

    fn supplied_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Root> {
        match expr.kind {
            ExprKind::Call(callee, arguments) => {
                if Self::interface_callable(cx, callee, 0)
                    && let Some(receiver) = arguments.first()
                {
                    return Some(match Self::root(cx, receiver, 0) {
                        Root::Supplied => Root::Supplied,
                        _ => Root::Unproven,
                    });
                }
                // A free function path is not an injected callback. Fn pointers,
                // closure parameters and supplied Fn fields have a local root.
                let root = Self::root(cx, callee, 0);
                (root != Root::Value).then_some(root)
            }
            ExprKind::MethodCall(_, receiver, _, _) => {
                let method = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
                // Ordinary argument/value methods are not services merely because
                // their receiver also happens to be a parameter.
                Self::interface(cx.tcx, method).then(|| match Self::root(cx, receiver, 0) {
                    Root::Supplied => Root::Supplied,
                    _ => Root::Unproven,
                })
            }
            _ => None,
        }
    }

    fn interface_callable<'tcx>(
        cx: &LateContext<'tcx>,
        expr: &'tcx Expr<'tcx>,
        depth: usize,
    ) -> bool {
        if depth > 64 {
            return false;
        }
        match expr.kind {
            ExprKind::AddrOf(_, _, value)
            | ExprKind::Unary(UnOp::Deref, value)
            | ExprKind::Cast(value, _) => Self::interface_callable(cx, value, depth + 1),
            ExprKind::Path(ref path) => match cx.qpath_res(path, expr.hir_id) {
                Res::Local(binding) => find_binding_init(cx, binding)
                    .is_some_and(|value| Self::interface_callable(cx, value, depth + 1)),
                resolved => resolved
                    .opt_def_id()
                    .is_some_and(|method| Self::interface(cx.tcx, method)),
            },
            _ => false,
        }
    }

    fn require_proof(&self, cx: &LateContext<'_>, command: DefId, span: Span, suffix: &str) {
        let name = format!("{}_{suffix}", cx.tcx.item_name(command));
        if !self
            .proofs
            .iter()
            .any(|(owner, proof)| *owner == command && *proof == name)
        {
            span_lint(
                cx,
                COMMAND_SERVICE_CONTRACT,
                span,
                format!("command requires an active #[test] `{name}` in its command_test module"),
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for CommandContract {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        _: &Body<'tcx>,
        span: Span,
        owner: LocalDefId,
    ) {
        if cx.tcx.def_kind(owner) != DefKind::Fn {
            return;
        }
        if let Some(command) = Self::command(cx.tcx, owner.to_def_id(), "command")
            && cx.tcx.item_name(owner.to_def_id()).as_str() == "run"
        {
            self.commands.push((command, span));
        }
        if let Some(command) = Self::command(cx.tcx, owner.to_def_id(), "command_test")
            && is_test_function(cx.tcx, owner)
            && !cx
                .tcx
                .hir_attrs(cx.tcx.local_def_id_to_hir_id(owner))
                .iter()
                .any(|attribute| {
                    matches!(
                        attribute,
                        Attribute::Parsed(
                            AttributeKind::Ignore { .. } | AttributeKind::ShouldPanic { .. }
                        )
                    )
                })
        {
            self.proofs
                .push((command, cx.tcx.item_name(owner.to_def_id()).to_string()));
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(command) = Self::command(cx.tcx, expr.hir_id.owner.def_id.to_def_id(), "command")
        else {
            return;
        };
        let Some(root) = Self::supplied_call(cx, expr) else {
            return;
        };
        if root == Root::Value {
            return;
        }
        let environment = TypingEnv::post_analysis(cx.tcx, expr.hir_id.owner.def_id);
        let value = cx.tcx.try_normalize_erasing_regions(
            environment,
            ty::Unnormalized::new_wip(cx.typeck_results().expr_ty(expr)),
        );
        let Ok(value) = value else {
            span_lint(
                cx,
                COMMAND_SERVICE_CONTRACT,
                expr.span,
                "supplied service output is unresolved; its proof obligation is not established",
            );
            return;
        };
        if root == Root::Unproven
            || matches!(
                value.kind(),
                ty::Param(_) | ty::Alias(_) | ty::Infer(_) | ty::Error(_)
            )
        {
            span_lint(
                cx,
                COMMAND_SERVICE_CONTRACT,
                expr.span,
                "supplied service root or output is unresolved; it cannot be classified as infallible",
            );
        } else if let ty::Adt(definition, _) = value.kind()
            && cx.tcx.is_diagnostic_item(sym::Result, definition.did())
            && !self.fallible.contains(&command)
        {
            self.fallible.push(command);
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Production checks still validate classification. The configured native
        // gate also compiles test targets, where proof functions actually exist.
        if !cx.tcx.sess.opts.test {
            return;
        }
        for &(command, span) in &self.commands {
            self.require_proof(cx, command, span, "preserves_supplied_service_contract");
            if self.fallible.contains(&command) {
                self.require_proof(cx, command, span, "fails_when_the_service_fails");
            }
        }
    }
}
