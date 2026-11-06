//! The application owns each invocation's mutable allocations and release boundary.
use super::relations::AdmittedInput;
use crate::domain::{
    derived_views::types::PreparedViewUse,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
};
use std::sync::Arc;

use crate::domain::operator::contract::{
    axis::AxisWorkspace,
    graph::{CycleMarks, DfsWorkspace},
};

use crate::domain::table_model::types::Rows;
use crate::domain::table_reach::types::Walk;
use crate::domain::table_scratch::types::{ScratchWord, Stretches};
use ryo_relations::domain::relations::types::{Position, Value};
pub(crate) struct InvocationWorkspace {
    pub work: crate::domain::operator::contract::types::WorkCounters,
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress_origin: super::route_counts::CapturedOrigin,
    pub derived_scratch: std::cell::RefCell<crate::domain::derived_views::types::DerivedScratch>,
    relation_owner: std::rc::Rc<super::relations::RelationOwner>,
    pub publications: Vec<Option<crate::application::file_dag::relations::PublishedInput>>,
    #[cfg(test)]
    pub observation: crate::domain::operating::types::ExecutionObservation,
    pub downstream_scratch: crate::domain::table_scratch::types::OperatorScratch,
    pub downstream_memo: Vec<crate::domain::operating::types::Memo>,
    pub predicates: crate::domain::table_scratch::types::LazyScratch<
        crate::domain::table_scratch::types::SelectScratch,
    >,
    pub predicate_memo: Vec<crate::domain::operating::types::Memo>,
    pub predicate_names: Option<crate::domain::operating::types::NameCount>,
    pub literal_regex: Option<ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    pub primitive: crate::domain::operator::contract::types::PrimitiveWorkspace,
}

impl InvocationWorkspace {
    pub(in crate::application::file_dag) fn for_contract(
        owner: std::rc::Rc<super::relations::RelationOwner>,
        contract: &crate::domain::operator::contract::types::OperatorContract,
    ) -> Self {
        let mut workspace = Self::new(owner);
        crate::domain::operator::contract::workspace::begin(
            &contract.workspace,
            crate::domain::operator::contract::types::WorkspaceLifetime::Invocation,
            &mut workspace.primitive,
        );
        workspace
    }
    pub(crate) fn new(owner: std::rc::Rc<super::relations::RelationOwner>) -> Self {
        Self {
            work: Default::default(),
            #[cfg(feature = "diagnostic-row-progress")]
            progress_origin: super::route_counts::capture(),
            derived_scratch: Default::default(),
            relation_owner: owner,
            publications: Default::default(),
            downstream_scratch: Default::default(),
            downstream_memo: Default::default(),
            predicates: Default::default(),
            predicate_memo: Default::default(),
            predicate_names: Default::default(),
            literal_regex: Default::default(),
            primitive: Default::default(),
            #[cfg(test)]
            observation: Default::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn measured(&self) -> crate::domain::operating::types::ExecutionObservation {
        let mut observed = self.observation;
        observed.regexes = self
            .predicates
            .0
            .get()
            .map_or(0, |state| state.compiled_regexes.0.len())
            + usize::from(self.literal_regex.is_some());
        observed.globs = self
            .predicates
            .0
            .get()
            .map_or(0, |state| state.compiled_globs.len());
        observed.row_capacity = self.primitive.row.capacity();
        observed.other_capacity = self.primitive.other.capacity();
        observed.closure_visited = self
            .primitive
            .closure_walk
            .as_ref()
            .map_or(0, |walk| walk.visited.len())
            .max(
                self.primitive
                    .axis
                    .as_ref()
                    .map_or(0, |axis| axis.walk.visited.len()),
            );
        observed.dfs_expanded = self.primitive.dfs.as_ref().map_or(0, |dfs| dfs.expanded);
        observed.dfs_examined = self.primitive.dfs.as_ref().map_or(0, |dfs| dfs.examined);
        observed.dfs_frames_len = self
            .primitive
            .dfs
            .as_ref()
            .map_or(0, |dfs| dfs.walk.frames.len());
        observed.dfs_visited_len = self
            .primitive
            .dfs
            .as_ref()
            .map_or(0, |dfs| dfs.walk.visited.len());
        observed
    }
}

impl InvocationWorkspace {
    pub(in crate::application::file_dag) fn release_inputs(&mut self) {
        self.publications.clear();
    }
    pub(in crate::application::file_dag) fn capture_admitted(
        &mut self,
        publications: Vec<super::relations::PublishedInput>,
    ) {
        self.publications = publications.into_iter().map(Some).collect();
    }

    pub(super) fn owner(&self) -> &super::relations::RelationOwner {
        &self.relation_owner
    }
    pub(super) fn owner_handle(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        std::rc::Rc::clone(&self.relation_owner)
    }
}

/// Only the workspace can bind an original graph row view to its admission.
/// The graph path never receives independently replaceable raw rows and proof.
pub(super) struct GraphInput<'a> {
    rows: &'a crate::domain::table_model::types::Rows<'a>,
    proof: AdmittedInput<'a>,
    relation: Option<ryo_relations::domain::relations::types::Relation>,
    owner: &'a super::relations::RelationOwner,
}
impl<'a> GraphInput<'a> {
    pub(super) fn admission(
        &self,
        plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AdmittedViewInputs<'a>,
        OperatingError,
    >{
        self.owner
            .derived
            .admit_view(plan, self.owner, &[&self.proof])
    }
    pub(super) fn rows(&self) -> &crate::domain::table_model::types::Rows<'a> {
        self.rows
    }
    pub(super) fn owner(&self) -> &super::derived::owner::DerivedOwner {
        &self.owner.derived
    }
    pub(super) fn canonical_backing(&self) -> bool {
        self.proof.canonical_backing()
    }
    pub(super) fn population(&self) -> Result<super::derived::owner::PopulationId, OperatingError> {
        self.proof.backing_population(self.owner)
    }
    pub(super) fn relation(&self) -> Option<ryo_relations::domain::relations::types::Relation> {
        self.relation
    }
    pub(super) fn generation(&self) -> Arc<()> {
        self.proof.generation_guard()
    }
}
impl InvocationWorkspace {
    pub(super) fn graph_input<'a>(
        &'a self,
        place: usize,
        rows: &'a crate::domain::table_model::types::Rows<'a>,
    ) -> Result<Option<GraphInput<'a>>, OperatingError> {
        let proof = self.admit(place, BorrowedRows::from(rows))?;
        match proof.physical_shape() {
            crate::domain::table_model::collections::representations::types::PhysicalShape::ReadablePhysicalFallback => Ok(None),
            crate::domain::table_model::collections::representations::types::PhysicalShape::Complete => {
                let relation = proof.graph_relation(self.owner())?;
                Ok(Some(GraphInput { rows, proof, relation, owner: self.owner() }))
            },
        }
    }
}

/// Sources are supplied only by admitted inputs, never independently by a view caller.
pub(super) enum ViewInputs<'a, 's> {
    Rows,
    MembershipValues,
    MembershipNative,
    Text(&'a crate::domain::table_texts::types::Texts<'a>),
    Dfs {
        symbols: &'a mut ryo_language::domain::symbols::types::QuerySymbols<'s>,
        source: ryo_values::domain::files::types::SourceView<'a>,
        paths: &'a [ryo_language::domain::files::types::FilePath],
        arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    },
}
/// One immutable demand scope, not a retained cache. At most three registered
/// operator inputs are held; only demanded slots acquire a proof.
pub(super) struct InputAdmissions<'a, 'f> {
    workspace: &'a InvocationWorkspace,
    inputs: &'a [BorrowedRows<'f>],
    proofs: [std::cell::OnceCell<AdmittedInput<'f>>; 3],
}
impl<'a, 'f> InputAdmissions<'a, 'f> {
    pub(super) fn owner_handle(&self) -> std::rc::Rc<super::relations::RelationOwner> {
        self.workspace.owner_handle()
    }
    pub(super) fn input(&self, place: usize) -> Result<BorrowedRows<'f>, OperatingError> {
        self.inputs
            .get(place)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: place + 1,
            })
    }
    pub(super) fn rows(&self, place: usize) -> Result<BorrowedRows<'f>, OperatingError> {
        Ok(self.proof(place)?.rows())
    }
    fn proof(&self, place: usize) -> Result<&AdmittedInput<'f>, OperatingError> {
        use OperatingError;
        let cell = self
            .proofs
            .get(place)
            .ok_or(OperatingError::WrongOperation)?;
        if cell.get().is_none() {
            let rows = *self.inputs.get(place).ok_or(OperatingError::MissingInput {
                position: place + 1,
            })?;
            cell.set(self.workspace.admit(place, rows)?)
                .map_err(|_| OperatingError::WrongOperation)?;
        }
        Ok(cell.get().expect("admitted demanded input"))
    }
    pub(super) fn partitioned(
        &self,
        order: &PreparedViewUse,
        closure: &PreparedViewUse,
        input: BorrowedRows<'f>,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::closure_partitions::PartitionedClosure<'f>, OperatingError> {
        let proof = self.proof(order.inputs[0])?;
        if !super::relations::same_admitted_rows(input, proof.rows()) {
            return Err(OperatingError::NotRead);
        }
        let storage = self.acquire_rows(order, context)?;
        super::closure_partitions::PartitionedClosure::from_prepared(
            proof.clone(),
            Arc::clone(&order.plan),
            Arc::clone(&closure.plan),
            storage,
            self.owner_handle(),
        )
    }
    pub(super) fn try_native_lookup(
        &self,
        use_: &PreparedViewUse,
    ) -> Result<
        Option<super::derived::owner::acquisition::admission::service::AcquiredView>,
        OperatingError,
    > {
        if use_.plan.kind != ryo_derived::domain::contract::types::DerivedKind::NativeLookup
            || use_.len != 2
        {
            return Err(OperatingError::WrongOperation);
        }
        let left = self.proof(use_.inputs[0])?;
        let right = self.proof(use_.inputs[1])?;
        let owner = self.workspace.owner();
        let admitted = owner
            .derived
            .admit_view(&use_.plan, owner, &[left, right])?;
        owner.derived.try_acquire_rows(
            &use_.plan,
            &admitted,
            crate::domain::derived_views::types::DerivedInputs::Rows(&[left.rows(), right.rows()]),
            &mut self.workspace.derived_scratch.borrow_mut(),
        )
    }
    pub(super) fn acquire_rows(
        &self,
        use_: &PreparedViewUse,
        context: &mut OperatorContext<'_>,
    ) -> Result<super::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>
    {
        self.acquire_derived(
            use_,
            super::derived::owner::ViewContext::default(),
            ViewInputs::Rows,
        )
    }
    pub(super) fn acquire_derived(
        &self,
        use_: &PreparedViewUse,
        identity: super::derived::owner::ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<super::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>
    {
        self.workspace
            .acquire_admitted(use_, self, identity, supplied)
    }
}
impl InvocationWorkspace {
    pub(super) fn inputs<'a, 'f>(
        &'a self,
        inputs: &'a [BorrowedRows<'f>],
    ) -> InputAdmissions<'a, 'f> {
        InputAdmissions {
            workspace: self,
            inputs,
            proofs: std::array::from_fn(|_| std::cell::OnceCell::new()),
        }
    }
    fn admit<'a>(
        &self,
        place: usize,
        rows: BorrowedRows<'a>,
    ) -> Result<AdmittedInput<'a>, OperatingError> {
        let owner = self.owner();
        if let Some(record) = self.publications.get(place).and_then(Option::as_ref) {
            return owner.admit(record.as_ref(), rows);
        }
        if place >= self.publications.len() {
            Err(OperatingError::MissingInput {
                position: place + 1,
            })
        } else {
            Err(OperatingError::NotRun)
        }
    }

    fn acquire_admitted(
        &self,
        use_: &PreparedViewUse,
        inputs: &InputAdmissions<'_, '_>,
        identity: super::derived::owner::ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<super::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>
    {
        use crate::domain::{
            derived_views::types::{DerivedInputs, MembershipSpace},
            operating::errors::OperatingError,
            operator::contract::views::BorrowedRows,
        };
        use ryo_derived::domain::contract::types::DerivedKind;
        let count = usize::from(use_.len);
        if !(1..=2).contains(&count) {
            return Err(OperatingError::WrongOperation);
        }
        if count < 2
            && matches!(
                &supplied,
                ViewInputs::MembershipNative | ViewInputs::Dfs { .. }
            )
        {
            return Err(OperatingError::MissingInput { position: 2 });
        }
        let mut proofs: [Option<&AdmittedInput<'_>>; 2] = [None, None];
        let mut relations = [None; 2];
        for at in 0..count {
            let proof = inputs.proof(use_.inputs[at])?;
            let (_, relation) = proof.identity(self.owner())?;
            relations[at] = relation;
            proofs[at] = Some(proof);
        }
        let rows = [
            proofs[0].as_ref().expect("first admitted input").rows(),
            proofs[1]
                .as_ref()
                .map_or(BorrowedRows::Computed(&EMPTY_DERIVED_INPUT), |proof| {
                    proof.rows()
                }),
        ];
        let supplied = match supplied {
            ViewInputs::Rows => DerivedInputs::Rows(&rows[..count]),
            ViewInputs::Text(texts) => DerivedInputs::Text {
                rows: &rows[..count],
                texts,
            },
            ViewInputs::MembershipValues => DerivedInputs::Membership {
                rows: rows[0],
                space: MembershipSpace::Values,
            },
            ViewInputs::MembershipNative => {
                let BorrowedRows::Tree(tree) = rows[1] else {
                    return Err(OperatingError::WrongOperation);
                };
                DerivedInputs::Membership {
                    rows: rows[0],
                    space: MembershipSpace::Native(tree),
                }
            }
            ViewInputs::Dfs {
                symbols,
                source,
                paths,
                arenas,
            } => DerivedInputs::Dfs {
                roots: rows[0],
                edges: rows[1],
                symbols,
                source,
                paths,
                arenas,
            },
        };
        let stored = if use_.plan.kind == DerivedKind::Equality && count == 1 {
            match rows[0] {
                BorrowedRows::Stored(source) => relations[0]
                    .or(use_.relations[0])
                    .map(|relation| (relation, source)),
                _ => None,
            }
        } else {
            None
        };
        let owner = &self.owner().derived;
        let sources = proofs[..count]
            .iter()
            .map(|proof| proof.expect("admitted input"))
            .collect::<Vec<_>>();
        let admitted = owner.admit_view(&use_.plan, self.owner(), &sources)?;
        owner.acquire_stored(
            &use_.plan,
            &admitted,
            identity,
            supplied,
            &mut self.derived_scratch.borrow_mut(),
            stored,
        )
    }
}
static EMPTY_DERIVED_INPUT: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: ryo_values::domain::rows::types::RowCount(0),
};

#[cfg(test)]
#[path = "workspace_test.rs"]
mod tests;

use ryo_values::domain::rows::types::{RowIndex, Table};
