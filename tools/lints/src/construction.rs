//! Compiler-resolved construction at composition boundaries.

use clippy_utils::{diagnostics::span_lint, get_parent_expr};
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::{Expr, ExprKind, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty, TyCtxt};

rustc_session::declare_lint! {
    pub COMPOSITION_CONSTRUCTION,
    Forbid,
    "composition wiring selects a concrete service or returns to a production builder"
}

pub(crate) struct Construction {
    macro_entries: Vec<LocalDefId>,
}
rustc_session::impl_lint_pass!(Construction => [COMPOSITION_CONSTRUCTION]);

impl Construction {
    pub(crate) fn new(tcx: TyCtxt<'_>) -> Self {
        if tcx
            .sess
            .opts
            .lint_cap
            .is_some_and(|level| !level.is_error())
            || tcx.sess.opts.lint_opts.iter().any(|(name, level)| {
                name.replace('-', "_") == COMPOSITION_CONSTRUCTION.name_lower()
                    && *level != rustc_session::lint::Level::Forbid
            })
        {
            tcx.sess
                .dcx()
                .err("composition checking requires an unweakened native lint level");
        }
        Self {
            macro_entries: tcx
                .resolutions(())
                .proc_macros
                .iter()
                .filter_map(|&registration| Self::macro_entry(tcx, registration))
                .collect(),
        }
    }

    // Registration comes from rustc, not a name, attribute spelling or crate label.
    // The macro spec requires an otherwise empty wrapper forwarding each argument.
    fn macro_entry(tcx: TyCtxt<'_>, registration: LocalDefId) -> Option<LocalDefId> {
        let body = tcx.hir_body_owned_by(registration);
        let ExprKind::Block(block, _) = body.value.kind else {
            return None;
        };
        if !block.stmts.is_empty() {
            return None;
        }
        let ExprKind::Call(callee, arguments) = block.expr?.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        let typeck = tcx.typeck(registration);
        let target = typeck.qpath_res(path, callee.hir_id).opt_def_id()?;
        let module = tcx.opt_parent(target)?;
        if !target.is_local()
            || tcx.def_kind(target) != DefKind::Fn
            || tcx.item_name(target) != tcx.item_name(registration.to_def_id())
            || tcx.def_kind(module) != DefKind::Mod
            || tcx.item_name(module).as_str() != "composition"
            || tcx.opt_parent(module) != tcx.opt_parent(registration.to_def_id())
            || arguments.len() != body.params.len()
        {
            return None;
        }
        let forwarded = arguments
            .iter()
            .zip(body.params)
            .all(|(argument, parameter)| {
                let PatKind::Binding(_, binding, _, None) = parameter.pat.kind else {
                    return false;
                };
                let ExprKind::Path(ref path) = argument.kind else {
                    return false;
                };
                matches!(typeck.qpath_res(path, argument.hir_id), Res::Local(id) if id == binding)
            });
        forwarded.then(|| target.expect_local())
    }

    fn macro_build<'tcx>(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
        // Do not collapse closure owners here: a callback inside the entry is not
        // itself the registered bootstrap. Function references are not calls either.
        let owner = cx.tcx.hir_enclosing_body_owner(expr.hir_id);
        if !self.macro_entries.contains(&owner) {
            return false;
        }
        let call = if matches!(expr.kind, ExprKind::Call(..)) {
            expr
        } else if let Some(parent) = get_parent_expr(cx, expr)
            && matches!(parent.kind, ExprKind::Call(callee, _) if callee.hir_id == expr.hir_id)
        {
            parent
        } else {
            return false;
        };
        let ExprKind::Call(callee, _) = call.kind else {
            return false;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return false;
        };
        cx.qpath_res(path, callee.hir_id)
            .opt_def_id()
            .is_some_and(|target| {
                target.is_local()
                    && Self::builder(cx.tcx, target)
                    && cx.tcx.opt_parent(target) == cx.tcx.opt_parent(owner.to_def_id())
            })
    }

    fn owner(tcx: TyCtxt<'_>, mut owner: LocalDefId) -> LocalDefId {
        while matches!(tcx.def_kind(owner), DefKind::Closure | DefKind::AnonConst) {
            owner = tcx.parent(owner.to_def_id()).expect_local();
        }
        owner
    }

    /// A `composition` module the CLI and SDK specs place as a composition root: one under
    /// `commands` or `entries` (`composition-wiring`'s include), or a
    /// proc-macro crate's, whose registered entries bootstrap it. A crate spec's own
    /// `composition.rs` (and a hex's) chooses its implementations by design and is not one.
    fn in_composition(tcx: TyCtxt<'_>, mut owner: DefId) -> bool {
        loop {
            if tcx.def_kind(owner) == DefKind::Mod && tcx.item_name(owner).as_str() == "composition"
            {
                return Self::root(tcx, owner);
            }
            let Some(parent) = tcx.opt_parent(owner) else {
                return false;
            };
            owner = parent;
        }
    }

    fn root(tcx: TyCtxt<'_>, composition: DefId) -> bool {
        if tcx
            .crate_types()
            .contains(&rustc_session::config::CrateType::ProcMacro)
        {
            return true;
        }
        let mut owner = composition;
        while let Some(parent) = tcx.opt_parent(owner) {
            if tcx.def_kind(parent) == DefKind::Mod
                && matches!(
                    tcx.opt_item_name(parent)
                        .as_ref()
                        .map(rustc_span::Symbol::as_str),
                    Some("commands" | "entries")
                )
            {
                return true;
            }
            owner = parent;
        }
        false
    }

    fn builder(tcx: TyCtxt<'_>, owner: DefId) -> bool {
        if !matches!(tcx.def_kind(owner), DefKind::Fn | DefKind::AssocFn) {
            return false;
        }
        let name = tcx.item_name(owner);
        name.as_str() == "build" || name.as_str().starts_with("build_")
    }

    fn definition_path(tcx: TyCtxt<'_>, mut id: DefId) -> Vec<String> {
        let mut segments = Vec::new();
        loop {
            if let Some(name) = tcx.def_key(id).disambiguated_data.data.get_opt_name() {
                segments.push(name.to_string());
            }
            let Some(parent) = tcx.opt_parent(id) else {
                break;
            };
            id = parent;
        }
        segments.push(tcx.crate_name(id.krate).to_string());
        segments.reverse();
        segments
    }

    fn provider_bootstrap(tcx: TyCtxt<'_>, owner: DefId) -> bool {
        if tcx.def_kind(owner) != DefKind::AssocFn || tcx.item_name(owner).as_str() != "construct" {
            return false;
        }
        let Some(trait_id) = tcx.trait_of_assoc(owner).or_else(|| {
            tcx.trait_item_of(owner)
                .and_then(|item| tcx.trait_of_assoc(item))
        }) else {
            return false;
        };
        tcx.crate_name(trait_id.krate).as_str() == "ryo_language"
            && Self::definition_path(tcx, trait_id)
                == ["ryo_language", "registration", "types", "ProviderFamily"]
    }

    fn implementation(tcx: TyCtxt<'_>, owner: DefId) -> bool {
        let segments = Self::definition_path(tcx, owner);
        segments
            .iter()
            .any(|name| matches!(name.as_str(), "adapters" | "use_case"))
    }

    fn selection(tcx: TyCtxt<'_>, owner: DefId) -> bool {
        let segments = Self::definition_path(tcx, owner);
        segments.iter().any(|name| name == "factory")
            || segments
                .windows(4)
                .any(|part| part[0] == "entries" && part[3] == "entry")
            || (Self::in_composition(tcx, owner) && Self::builder(tcx, owner))
    }

    fn checked_helper(tcx: TyCtxt<'_>, owner: DefId) -> bool {
        owner.is_local() && Self::in_composition(tcx, owner) && !Self::builder(tcx, owner)
    }

    fn constructed<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> bool {
        match value.kind() {
            ty::Adt(definition, arguments) => {
                Self::implementation(tcx, definition.did())
                    || arguments.types().any(|inner| Self::constructed(tcx, inner))
            }
            ty::Tuple(values) => values.iter().any(|inner| Self::constructed(tcx, inner)),
            ty::Alias(alias) if matches!(alias.kind, ty::Opaque { .. }) => tcx
                .explicit_item_bounds(alias.kind.def_id())
                .iter_instantiated_copied(tcx, alias.args)
                .map(|clause| clause.skip_norm_wip())
                .any(|(clause, _)| {
                    matches!(clause.kind().skip_binder(), ty::ClauseKind::Trait(predicate)
                    if tcx.item_name(predicate.def_id()).as_str().ends_with("Interface"))
                }),
            ty::Dynamic(predicates, _) => predicates.iter().any(|predicate| {
                matches!(predicate.skip_binder(), ty::ExistentialPredicate::Trait(trait_ref)
                    if tcx.item_name(trait_ref.def_id).as_str().ends_with("Interface"))
            }),
            // References and parameter values are dependencies, not owned construction.
            _ => false,
        }
    }

    fn default(tcx: TyCtxt<'_>, id: DefId) -> bool {
        Self::definition_path(tcx, id) == ["core", "default", "Default", "default"]
            || tcx.trait_of_assoc(id).is_some_and(|item| {
                Self::definition_path(tcx, item) == ["core", "default", "Default"]
            })
    }
}

impl<'tcx> LateLintPass<'tcx> for Construction {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let owner = Self::owner(cx.tcx, expr.hir_id.owner.def_id).to_def_id();
        if !Self::in_composition(cx.tcx, owner)
            || Self::builder(cx.tcx, owner)
            || Self::provider_bootstrap(cx.tcx, owner)
            || self.macro_build(cx, expr)
        {
            return;
        }
        let rejected = match expr.kind {
            ExprKind::Path(ref path) => {
                cx.qpath_res(path, expr.hir_id)
                    .opt_def_id()
                    .is_some_and(|id| {
                        let default_reference = Self::default(cx.tcx, id)
                            && matches!(cx.typeck_results().expr_ty(expr).kind(), ty::FnDef(definition, arguments)
                                if Self::constructed(cx.tcx, cx.tcx.fn_sig(*definition).instantiate(cx.tcx, arguments).skip_binder().output()));
                        Self::selection(cx.tcx, id)
                            || default_reference
                            || (matches!(
                                cx.tcx.def_kind(id),
                                DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(..)
                            ) && (Self::implementation(cx.tcx, id)
                                || (matches!(cx.tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(_, CtorKind::Fn))
                                    && !Self::checked_helper(cx.tcx, id) && !matches!(
                                    cx.tcx.crate_name(id.krate).as_str(),
                                    "core" | "alloc" | "std"
                                ) && Self::constructed(
                                    cx.tcx,
                                    cx.tcx
                                        .fn_sig(id)
                                        .instantiate_identity()
                                        .skip_binder()
                                        .output(),
                                ))))
                    })
            }
            // Inspect a resolved constructor call, never a supplied closure/function pointer.
            // Pure container wrapping/cloning does not select the dependency it receives.
            ExprKind::Call(callee, _) => {
                if let ExprKind::Path(ref path) = callee.kind {
                    cx.qpath_res(path, callee.hir_id)
                        .opt_def_id()
                        .is_some_and(|id| {
                            let default = Self::default(cx.tcx, id);
                            let crate_name = cx.tcx.crate_name(id.krate);
                            let owned_call = !Self::checked_helper(cx.tcx, id)
                                && !matches!(crate_name.as_str(), "core" | "alloc" | "std");
                            (default || owned_call)
                                && Self::constructed(cx.tcx, cx.typeck_results().expr_ty(expr))
                        })
                } else {
                    false
                }
            }
            ExprKind::Struct(..) => Self::constructed(cx.tcx, cx.typeck_results().expr_ty(expr)),
            ExprKind::MethodCall(..) => cx
                .typeck_results()
                .type_dependent_def_id(expr.hir_id)
                .is_some_and(|id| Self::selection(cx.tcx, id)),
            _ => false,
        };
        if rejected {
            span_lint(
                cx,
                COMPOSITION_CONSTRUCTION,
                expr.span,
                "composition must use supplied dependencies and service constructors; production selection belongs in build",
            );
        }
    }
}
