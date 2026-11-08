//! Only this private issuer binds actual source proofs and owner admission.
use super::super::*;
use crate::application::file_dag::derived::owner::DerivedOwner;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
impl AcquisitionOrigin {
    pub(crate) fn validate(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
    ) -> Result<(), OperatingError> {
        if !Arc::ptr_eq(&self.owner, &owner.identity) || self.live.is_live() != crate::application::file_dag::relations::types::Liveness::Live {
            return Err(OperatingError::NotRead);
        }
        if !Arc::ptr_eq(&self.plan, plan) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
}

impl AdmittedViewInputs<'_> {
    pub(crate) fn validate(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
        inputs: &DerivedInputs<'_, '_>,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, plan)?;

use ryo_execution_domain::domain::table_model::types::RepresentationInput;
        let admitted_rows = match (&self.sources, inputs) {
            (
                AcquisitionSources::Rows(_),
                DerivedInputs::Membership {
                    space: MembershipSpace::Dense(_),
                    ..
                },
            ) => {
                return Err(OperatingError::NotRead);
            }
            (AcquisitionSources::Rows(rows), _) => rows,
            (
                AcquisitionSources::Dense { rows, graph },
                DerivedInputs::Membership {
                    space: MembershipSpace::Dense(values),
                    ..
                },
            ) if std::ptr::eq(graph.vertices(), *values) => rows,
            _ => return Err(OperatingError::NotRead),
        };
        let rows = match inputs {
            DerivedInputs::Rows(rows) | DerivedInputs::Text { rows, .. } => rows.to_vec(),
            DerivedInputs::Partition { rows, .. }
            | DerivedInputs::Adjacency { rows, .. } => vec![*rows],
            DerivedInputs::Membership {
                rows,
                space: MembershipSpace::Native(tree),
            } => vec![*rows, RepresentationInput::Tree(*tree)],
            DerivedInputs::Membership { rows, .. } => vec![*rows],
            DerivedInputs::Dfs { roots, edges, .. } => vec![*roots, *edges],
        };
        if rows.len() != admitted_rows.len()
            || !rows
                .into_iter()
                .zip(admitted_rows)
                .all(|(actual, admitted)| {
                    crate::application::file_dag::relations::same_admitted_rows(actual, *admitted)
                        == crate::application::file_dag::relations::types::Sameness::Same
                })
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
}

impl DerivedOwner {
    pub(crate) fn admit_view<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        owner: &crate::application::file_dag::relations::RelationOwner,
        proofs: &[&crate::application::file_dag::relations::admission::service::BorrowedInput<
            'a,
        >],
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if !std::ptr::eq(self, owner.derived()) || proofs.is_empty() || proofs.len() > 2 {
            return Err(OperatingError::NotRead);
        }
        let populations = proofs
            .iter()
            .map(|proof| proof.identity(owner).map(|identity| identity.population))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AdmittedViewInputs {
            origin: AcquisitionOrigin {
                owner: Arc::clone(&self.identity),
                live: owner.generation_liveness(),
                plan: Arc::clone(plan),
                populations,
                lifetime: AcquisitionLifetime::Population,
            },
            sources: AcquisitionSources::Rows(proofs.iter().map(|proof| proof.rows()).collect()),
        })
    }
    pub(crate) fn admit_partition<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        owner: &crate::application::file_dag::relations::RelationOwner,
        proof: &crate::application::file_dag::relations::admission::service::BorrowedInput<'a>,
        parent: &AcquiredView,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        parent.origin().validate(self, &parent.origin().plan)?;
        if parent.contract().kind != DerivedKind::PartitionOrder
            || plan.kind != DerivedKind::ClosureAdjacency
        {
            return Err(OperatingError::WrongOperation);
        }
        let mut admitted = self.admit_view(plan, owner, &[proof])?;
        if admitted.origin.populations != parent.origin().populations {
            return Err(OperatingError::NotRead);
        }
        admitted.origin.lifetime = AcquisitionLifetime::Invocation;
        Ok(admitted)
    }
}

impl DerivedOwner {
    fn graph_sources<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        let mut admitted = input.admission(plan)?;
        admitted.origin.populations = vec![input.population()?];
        if input.canonical_backing() == crate::application::file_dag::relations::types::BackingForm::Foreign {
            admitted.origin.lifetime = AcquisitionLifetime::Invocation;
        }
        Ok(admitted)
    }
    pub(crate) fn admit_graph<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if plan.kind != DerivedKind::Adjacency {
            return Err(OperatingError::WrongOperation);
        }
        self.graph_sources(plan, input)
    }
    pub(crate) fn admit_domain<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
        parent: &AcquiredView,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if plan.kind != DerivedKind::Membership {
            return Err(OperatingError::WrongOperation);
        }
        parent.origin().validate(self, &parent.origin().plan)?;
        let mut admitted = self.graph_sources(plan, input)?;
        if !Arc::ptr_eq(&parent.origin().live, &admitted.origin.live) {
            return Err(OperatingError::NotRead);
        }
        let graph = parent
            .storage()
            .payload()
            .share_adjacency()
            .map(|shared| shared.0)
            .ok_or(OperatingError::WrongOperation)?;
        let AcquisitionSources::Rows(rows) = admitted.sources else {
            return Err(OperatingError::WrongOperation);
        };
        admitted.sources = AcquisitionSources::Dense { rows, graph };
        let population = input.population()?;
        admitted.origin.populations = parent.origin().populations.clone();
        if !admitted.origin.populations.contains(&population) {
            admitted.origin.populations.push(population);
        }
        Ok(admitted)
    }
}

impl AdmittedViewInputs<'_> {
    pub(crate) fn lifetime(
        &self,
    ) -> AcquisitionLifetime {
        self.origin.lifetime
    }
    pub(crate) fn validate_origin(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, plan)
    }
}

impl DerivedOwner {
    pub(crate) fn complete_view(
        &self,
        plan: &Arc<DerivedContract>,
        storage: Arc<DerivedStorage>,
        lease: Arc<ResourceLease>,
        backings: Arc<[Backing]>,
        admitted: &AdmittedViewInputs<'_>,
        built: Construction,
    ) -> Result<AcquiredView, OperatingError> {
        admitted.origin.validate(self, plan)?;
        storage.view()?;
        Ok(AcquiredView::new(
            plan,
            storage,
            lease,
            backings,
            admitted.origin.clone(),
            built,
        ))
    }
}

impl AcquiredView {
    pub(crate) fn validate_owner(
        &self,
        owner: &DerivedOwner,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, &self.plan)
    }
}
