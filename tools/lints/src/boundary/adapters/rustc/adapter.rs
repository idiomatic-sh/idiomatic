use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint;
use clippy_utils::paths::PathNS;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, DefIdMap, LocalDefId};
use rustc_hir::{ImplItem, ImplItemKind, Item, ItemKind, PrimTy, TraitItem, TraitItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt, TypingEnv};
use rustc_span::Span;
use serde::Deserialize as _;
use serde::de::IntoDeserializer as _;

use crate::boundary::types::{BoundarySite, REPRESENTATIONS};

const INTERFACE_MODULE: &str = "interface";
/// The suffixes of a trait that crosses a boundary: the retired `Interface` of the crates not yet moved, and the `Port` and `UseCase` of the parts.
const INTERFACE_SUFFIXES: [&str; 3] = ["Interface", "Port", "UseCase"];

rustc_session::declare_lint! {
    pub NOMINAL_BOUNDARY_TYPE,
    Forbid,
    "a compiler-resolved interface boundary exposes a configured representation type"
}
rustc_session::declare_lint! {
    pub UNPROVEN_BOUNDARY_TYPE,
    Forbid,
    "a concrete interface implementation has an unproven boundary type"
}

pub(crate) struct BoundaryRustcAdapter {
    definitions: DefIdMap<(&'static str, &'static DisallowedPath)>,
    primitives: FxHashMap<PrimTy, (&'static str, &'static DisallowedPath)>,
}

rustc_session::impl_lint_pass!(BoundaryRustcAdapter => [NOMINAL_BOUNDARY_TYPE, UNPROVEN_BOUNDARY_TYPE]);

impl BoundaryRustcAdapter {
    pub(crate) fn new(context: TyCtxt<'_>) -> Self {
        let names = [
            NOMINAL_BOUNDARY_TYPE.name_lower(),
            UNPROVEN_BOUNDARY_TYPE.name_lower(),
        ];
        if context
            .sess
            .opts
            .lint_cap
            .is_some_and(|level| !level.is_error())
            || context.sess.opts.lint_opts.iter().any(|(name, level)| {
                names.contains(&name.replace('-', "_"))
                    && *level != rustc_session::lint::Level::Forbid
            })
        {
            context
                .sess
                .dcx()
                .err("nominal checking requires unweakened native lint levels");
        }
        let policy: Vec<DisallowedPath> = REPRESENTATIONS
            .iter()
            .filter_map(|path| {
                DisallowedPath::deserialize(path.into_deserializer())
                    .map_err(|error: serde::de::value::Error| {
                        context.sess.dcx().err(format!(
                            "nominal policy entry `{path}` is malformed: {error}"
                        ));
                    })
                    .ok()
            })
            .collect();
        let policy: &'static [DisallowedPath] = Vec::leak(policy);
        let (definitions, primitives) = create_disallowed_map(
            context,
            policy,
            PathNS::Type,
            |kind| matches!(kind, DefKind::Struct | DefKind::Enum | DefKind::Union),
            "type",
            true,
        );
        for entry in policy {
            if !definitions
                .items()
                .any(|(_, (name, _))| *name == entry.path())
                && !primitives.values().any(|(name, _)| *name == entry.path())
            {
                context.sess.dcx().err(format!(
                    "nominal policy entry `{}` must resolve to a primitive or nominal type definition in this crate",
                    entry.path()
                ));
            }
        }
        Self {
            definitions,
            primitives,
        }
    }

    pub(crate) fn selected(context: TyCtxt<'_>, owner: DefId) -> bool {
        context.def_kind(owner) == DefKind::Trait
            && context.def_kind(context.parent(owner)) == DefKind::Mod
            && context.item_name(context.parent(owner)).as_str() == INTERFACE_MODULE
            && INTERFACE_SUFFIXES
                .iter()
                .any(|suffix| context.item_name(owner).as_str().ends_with(suffix))
    }

    fn implementation(context: TyCtxt<'_>, owner: LocalDefId) -> bool {
        matches!(context.def_kind(owner), DefKind::Impl { of_trait: true })
            && Self::selected(
                context,
                context
                    .impl_trait_ref(owner)
                    .instantiate_identity()
                    .skip_norm_wip()
                    .def_id,
            )
    }

    fn signature<'tcx>(
        &self,
        context: &LateContext<'tcx>,
        owner: LocalDefId,
        span: Span,
        site: BoundarySite,
    ) {
        let signature = context
            .tcx
            .fn_sig(owner)
            .instantiate_identity()
            .skip_binder();
        let inputs = if context.tcx.associated_item(owner).is_method() {
            &signature.inputs()[1..]
        } else {
            signature.inputs()
        };
        for value in inputs
            .iter()
            .copied()
            .chain(std::iter::once(signature.output()))
        {
            self.check(context, owner, span, value, site);
        }
    }

    fn nominal_receiver<'tcx>(
        &self,
        context: TyCtxt<'tcx>,
        owner: LocalDefId,
        value: Ty<'tcx>,
        visiting: &mut FxHashSet<DefId>,
    ) -> bool {
        match value.kind() {
            ty::Adt(definition, _) => !self.definitions.contains_key(&definition.did()),
            ty::Ref(_, element, _) => self.nominal_receiver(context, owner, *element, visiting),
            ty::Param(_) if value.is_sized(context, TypingEnv::post_analysis(context, owner)) => {
                context
                    .predicates_of(owner)
                    .predicates
                    .iter()
                    .any(|(clause, _)| {
                        let Some(bound) = clause.as_trait_clause() else {
                            return false;
                        };
                        let bound = bound.skip_binder();
                        let trait_id = bound.trait_ref.def_id;
                        if bound.polarity != ty::PredicatePolarity::Positive
                            || bound.trait_ref.self_ty() != value
                            || !trait_id.is_local()
                            || context.trait_def(trait_id).has_auto_impl
                            || context.generics_of(trait_id).count() != 1
                            || !visiting.insert(trait_id)
                        {
                            return false;
                        }
                        // Orphan rules close downstream implementations; inspect the owner's
                        // implementations too, including blankets that could admit primitives.
                        let implementations = context.trait_impls_of(trait_id);
                        let nominal = implementations
                            .blanket_impls()
                            .iter()
                            .chain(implementations.non_blanket_impls().values().flatten())
                            .all(|implementation| {
                                implementation.as_local().is_some_and(|implementation| {
                                    self.nominal_receiver(
                                        context,
                                        implementation,
                                        context
                                            .type_of(implementation)
                                            .instantiate_identity()
                                            .skip_norm_wip(),
                                        visiting,
                                    )
                                })
                            });
                        visiting.remove(&trait_id);
                        nominal
                    })
            }
            _ => false,
        }
    }

    fn check<'tcx>(
        &self,
        context: &LateContext<'tcx>,
        owner: LocalDefId,
        span: Span,
        value: Ty<'tcx>,
        site: BoundarySite,
    ) {
        let environment = TypingEnv::post_analysis(context.tcx, owner);
        let Ok(value) = context
            .tcx
            .try_normalize_erasing_regions(environment, ty::Unnormalized::new_wip(value))
        else {
            span_lint(
                context,
                UNPROVEN_BOUNDARY_TYPE,
                span,
                "compiler normalization could not establish the interface boundary type",
            );
            return;
        };
        let primitive = match value.kind() {
            ty::Bool => Some(PrimTy::Bool),
            ty::Char => Some(PrimTy::Char),
            ty::Str => Some(PrimTy::Str),
            ty::Int(kind) => Some(PrimTy::Int(*kind)),
            ty::Uint(kind) => Some(PrimTy::Uint(*kind)),
            ty::Float(kind) => Some(PrimTy::Float(*kind)),
            _ => None,
        };
        if let Some(primitive) = primitive {
            if let Some((name, _)) = self.primitives.get(&primitive) {
                span_lint(
                    context,
                    NOMINAL_BOUNDARY_TYPE,
                    span,
                    format!("interface boundary resolves to disallowed type `{name}`"),
                );
            }
            return;
        }
        match value.kind() {
            ty::Adt(definition, arguments) => {
                if let Some((name, _)) = self.definitions.get(&definition.did()) {
                    span_lint(
                        context,
                        NOMINAL_BOUNDARY_TYPE,
                        span,
                        format!("interface boundary resolves to disallowed type `{name}`"),
                    );
                } else if !matches!(site, BoundarySite::Receiver) {
                    for argument in arguments.types() {
                        self.check(context, owner, span, argument, site);
                    }
                }
            }
            ty::Ref(_, element, _)
            | ty::RawPtr(element, _)
            | ty::Array(element, _)
            | ty::Slice(element) => {
                self.check(context, owner, span, *element, site);
            }
            ty::Tuple(elements) => {
                for element in *elements {
                    self.check(context, owner, span, element, site);
                }
            }
            ty::FnPtr(..) => {
                let signature = value.fn_sig(context.tcx).skip_binder();
                for element in signature.inputs_and_output {
                    self.check(context, owner, span, element, site);
                }
            }
            ty::Dynamic(predicates, _) => {
                for predicate in predicates.iter() {
                    match predicate.skip_binder() {
                        ty::ExistentialPredicate::Trait(reference) => {
                            for argument in reference.args.types() {
                                self.check(context, owner, span, argument, site);
                            }
                        }
                        ty::ExistentialPredicate::Projection(projection) => {
                            if let Some(element) = projection.term.as_type() {
                                self.check(context, owner, span, element, site);
                            }
                        }
                        ty::ExistentialPredicate::AutoTrait(_) => {}
                    }
                }
            }
            ty::Never => {}
            ty::Param(_) if matches!(site, BoundarySite::Declaration) => {}
            // A parameter the implementing method declares itself repeats the
            // interface's own declaration: the boundary is the interface's, proven
            // there. A parameter of the implementation (`impl<T> ... for Wrapper<T>`)
            // is the implementation's and stays unproven.
            ty::Param(parameter)
                if matches!(site, BoundarySite::Implementation)
                    && parameter.index
                        >= u32::try_from(context.tcx.generics_of(owner).parent_count)
                            .unwrap_or(u32::MAX) => {}
            ty::Param(_)
                if matches!(site, BoundarySite::Receiver)
                    && self.nominal_receiver(
                        context.tcx,
                        owner,
                        value,
                        &mut FxHashSet::default(),
                    ) => {}
            ty::Alias(ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            }) if matches!(site, BoundarySite::Declaration)
                && value.has_param()
                && !context.tcx.is_impl_trait_in_trait(*def_id) => {}
            _ => span_lint(
                context,
                UNPROVEN_BOUNDARY_TYPE,
                span,
                format!("cannot establish a nominal implementation boundary for `{value}`"),
            ),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for BoundaryRustcAdapter {
    fn check_trait_item(&mut self, context: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let owner = item.owner_id.def_id;
        if !Self::selected(context.tcx, context.tcx.parent(owner.to_def_id())) {
            return;
        }
        match item.kind {
            TraitItemKind::Fn(..) => {
                self.signature(context, owner, item.span, BoundarySite::Declaration)
            }
            TraitItemKind::Const(..) => self.check(
                context,
                owner,
                item.span,
                context
                    .tcx
                    .type_of(owner)
                    .instantiate_identity()
                    .skip_norm_wip(),
                BoundarySite::Declaration,
            ),
            TraitItemKind::Type(..) => {}
        }
    }

    fn check_item(&mut self, context: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if matches!(item.kind, ItemKind::Impl(..))
            && Self::implementation(context.tcx, item.owner_id.def_id)
        {
            self.check(
                context,
                item.owner_id.def_id,
                item.span,
                context
                    .tcx
                    .type_of(item.owner_id.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
                BoundarySite::Receiver,
            );
        }
    }

    fn check_impl_item(&mut self, context: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let owner = item.owner_id.def_id;
        let implementation = context
            .tcx
            .impl_of_assoc(owner.to_def_id())
            .expect("impl item owner");
        if !Self::implementation(context.tcx, implementation.expect_local()) {
            return;
        }
        match item.kind {
            ImplItemKind::Fn(..) => {
                self.signature(context, owner, item.span, BoundarySite::Implementation)
            }
            ImplItemKind::Type(..) | ImplItemKind::Const(..) => self.check(
                context,
                owner,
                item.span,
                context
                    .tcx
                    .type_of(owner)
                    .instantiate_identity()
                    .skip_norm_wip(),
                BoundarySite::Implementation,
            ),
        }
    }
}
