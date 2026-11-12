//! The invocation workspace admits an operator's inputs and acquires the derived view they demand.
use crate::application::file_dag::relations::admission::service::BorrowedInput;
use ryo_execution_domain::domain::operating::errors::OperatingError;

use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_values::domain::plan_parts::types::InputNumber;

impl InvocationWorkspace {
    /// The admissions of `inputs` for this invocation.
    pub(crate) fn inputs<'a, 'f>(
        &'a self,
        inputs: &'a [RepresentationInput<'f>],
    ) -> InputAdmissions<'a, 'f> {
        InputAdmissions {
            workspace: self,
            inputs,
            proofs: std::array::from_fn(|_| std::cell::OnceCell::new()),
        }
    }
    /// Admits `rows` as the borrowed input at `place`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::MissingInput`] for a place beyond the captured publications, [`OperatingError::NotRun`] for a
    /// place whose publication was not captured, and the owner's refusal to admit `rows` for it.
    pub(crate) fn admit<'a>(
        &self,
        place: InputNumber,
        rows: RepresentationInput<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        let owner = self.owner();
        if let Some(record) = self.publications.get(place.0).and_then(Option::as_ref) {
            return owner.admit(record.input()?.as_ref(), rows);
        }
        if place.0 >= self.publications.len() {
            Err(OperatingError::MissingInput {
                position: place.0 + 1,
            })
        } else {
            Err(OperatingError::NotRun)
        }
    }
}

/// One invocation's mutable allocations and the admitted publications it reads, released when it ends.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::workspace::service::InvocationWorkspace;
/// fn named(_: Option<&InvocationWorkspace>) {}
/// ```
pub struct InvocationWorkspace {
    work: ryo_execution_domain::domain::operator::contract::types::WorkCounters,
    #[cfg(feature = "diagnostic-row-progress")]
    progress_origin: super::super::route_counts::profile::service::CapturedOrigin,
    pub(crate) derived_scratch:
        std::cell::RefCell<ryo_execution_domain::domain::derived_views::types::DerivedScratch>,
    relation_owner: std::rc::Rc<super::super::relations::service::RelationOwner>,
    pub(super) publications:
        Vec<Option<crate::application::file_dag::relations::handle::service::Publication>>,
    downstream: ryo_execution_domain::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers,
    predicates: ryo_execution_domain::domain::table_scratch::types::LazyScratch<
        ryo_execution_domain::domain::table_scratch::types::SelectScratch,
    >,
    predicate_memo: Vec<ryo_execution_domain::domain::operating::types::Memo>,
    predicate_names: Option<ryo_execution_domain::domain::operating::types::NameCount>,
    literal_regex: Option<ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    primitive: ryo_execution_domain::domain::operator::contract::types::PrimitiveWorkspace,
}

impl InvocationWorkspace {
    /// The workspace of one invocation of `contract`, its allocations begun for the invocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
    /// use ryo_execution_store::application::file_dag::workspace::service::InvocationWorkspace;
    /// use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
    /// use std::rc::Rc;
    /// fn workspace(owner: &Rc<RelationOwner>, contract: &OperatorContract) -> InvocationWorkspace {
    ///     InvocationWorkspace::for_contract(Rc::clone(owner), contract)
    /// }
    /// ```
    pub fn for_contract(
        owner: std::rc::Rc<super::super::relations::service::RelationOwner>,
        contract: &ryo_execution_domain::domain::operator::contract::types::OperatorContract,
    ) -> Self {
        let mut workspace = Self::new(owner);
        ryo_execution_domain::domain::operator::contract::workspace::service::begin(
            &contract.workspace,
            ryo_execution_domain::domain::operator::contract::types::WorkspaceLifetime::Invocation,
            &mut workspace.primitive,
        );
        workspace
    }
    pub(crate) fn new(owner: std::rc::Rc<super::super::relations::service::RelationOwner>) -> Self {
        Self {
            work: Default::default(),
            #[cfg(feature = "diagnostic-row-progress")]
            progress_origin: super::super::route_counts::service::capture(owner.routes()),
            derived_scratch: Default::default(),
            relation_owner: owner,
            publications: Default::default(),
            downstream: Default::default(),
            predicates: Default::default(),
            predicate_memo: Default::default(),
            predicate_names: Default::default(),
            literal_regex: Default::default(),
            primitive: Default::default(),
        }
    }

    /// What the invocation's allocations measure now, for the proofs of their reuse and release.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn measured(
        &self,
    ) -> ryo_execution_domain::domain::operating::fixtures::ExecutionObservation {
        let mut observed =
            ryo_execution_domain::domain::operating::fixtures::ExecutionObservation::default();
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
        observed.row_capacity = self.primitive.evaluation.rows.row.capacity();
        observed.other_capacity = self.primitive.evaluation.rows.other.capacity();
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
        observed.dfs_expanded = self.primitive.dfs.as_ref().map_or(0, |dfs| dfs.expanded.0);
        observed.dfs_examined = self.primitive.dfs.as_ref().map_or(0, |dfs| dfs.examined.0);
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
    /// Releases the admitted publications.
    pub(crate) fn release_inputs(&mut self) {
        self.publications.clear();
    }
    /// Holds `publications` as the invocation's admitted inputs.
    pub(crate) fn capture_admitted(
        &mut self,
        publications: Vec<super::super::relations::handle::service::Publication>,
    ) {
        self.publications = publications.into_iter().map(Some).collect();
    }

    /// The relation owner the invocation reads through.
    pub(crate) fn owner(&self) -> &super::super::relations::service::RelationOwner {
        &self.relation_owner
    }

    /// The rows each input was found to hold, counted as the kernels report them.
    pub(crate) fn work(
        &self,
    ) -> &ryo_execution_domain::domain::operator::contract::types::WorkCounters {
        &self.work
    }
    /// The route profile position this invocation was opened at.
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) fn progress_origin(
        &self,
    ) -> &super::super::route_counts::profile::service::CapturedOrigin {
        &self.progress_origin
    }
    /// The primitive operators' working allocations, to read what they hold.
    pub(crate) fn primitive(
        &self,
    ) -> &ryo_execution_domain::domain::operator::contract::types::PrimitiveWorkspace {
        &self.primitive
    }
    /// The primitive operators' working allocations, lent to a kernel.
    pub(crate) fn primitive_mut(
        &mut self,
    ) -> &mut ryo_execution_domain::domain::operator::contract::types::PrimitiveWorkspace {
        &mut self.primitive
    }
    /// Every working allocation a kernel uses, each lent apart from the others so one evaluation can hold them
    /// at once; the admitted publications stay with the workspace.
    pub(crate) fn parts(&mut self) -> super::types::WorkspaceParts<'_> {
        super::types::WorkspaceParts {
            owner: &self.relation_owner,
            work: &self.work,
            literal_regex: &mut self.literal_regex,
            predicates: &mut self.predicates,
            predicate_memo: &mut self.predicate_memo,
            predicate_names: &mut self.predicate_names,
            primitive: &mut self.primitive,
            downstream: &mut self.downstream,
        }
    }
    pub(crate) fn owner_handle(
        &self,
    ) -> std::rc::Rc<super::super::relations::service::RelationOwner> {
        std::rc::Rc::clone(&self.relation_owner)
    }
}

/// Only the workspace can bind an original graph row view to its admission.
/// The graph path never receives independently replaceable raw rows and proof.
pub(crate) struct GraphInput<'a> {
    rows: &'a ryo_execution_domain::domain::table_model::types::Rows<'a>,
    pub(crate) proof: BorrowedInput<'a>,
    relation: Option<ryo_relations::domain::relations::types::Relation>,
    pub(crate) owner: &'a super::super::relations::service::RelationOwner,
}

impl<'a> GraphInput<'a> {
    /// The rows of the graph input.
    pub(crate) fn rows(&self) -> &ryo_execution_domain::domain::table_model::types::Rows<'a> {
        self.rows
    }
    /// The derived-view owner of the graph input.
    pub(crate) fn owner(&self) -> &super::super::derived::owner::service::DerivedOwner {
        self.owner.derived()
    }
    pub(crate) fn canonical_backing(&self) -> super::super::relations::types::BackingForm {
        self.proof.canonical_backing()
    }
    /// The population the graph input was admitted under.
    pub(crate) fn population(
        &self,
    ) -> Result<super::super::derived::population::service::PopulationId, OperatingError> {
        self.proof.backing_population(self.owner)
    }
    /// The source relation of the graph input, when it came from one.
    pub(crate) fn relation(&self) -> Option<ryo_relations::domain::relations::types::Relation> {
        self.relation
    }
}

impl InvocationWorkspace {
    pub(crate) fn graph_input<'a>(
        &'a self,
        place: InputNumber,
        rows: &'a ryo_execution_domain::domain::table_model::types::Rows<'a>,
    ) -> Result<Option<GraphInput<'a>>, OperatingError> {
        let proof = self.admit(place, RepresentationInput::from(rows))?;
        match proof.physical_shape() {
            ryo_execution_domain::domain::table_model::types::PhysicalShape::ReadablePhysicalFallback => Ok(None),
            ryo_execution_domain::domain::table_model::types::PhysicalShape::Complete => {
                let relation = proof.graph_relation(self.owner())?;
                Ok(Some(GraphInput { rows, proof, relation, owner: self.owner() }))
            },
        }
    }
}

/// One immutable demand scope, not a retained cache. At most three registered
/// operator inputs are held; only demanded slots acquire a proof.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
/// fn named(_: Option<&InputAdmissions<'_, '_>>) {}
/// ```
pub struct InputAdmissions<'a, 'f> {
    pub(crate) workspace: &'a InvocationWorkspace,
    inputs: &'a [RepresentationInput<'f>],
    proofs: [std::cell::OnceCell<BorrowedInput<'f>>; 3],
}

impl<'f> InputAdmissions<'_, 'f> {
    pub(crate) fn owner_handle(
        &self,
    ) -> std::rc::Rc<super::super::relations::service::RelationOwner> {
        self.workspace.owner_handle()
    }
    /// The input at `place`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::MissingInput`] when the invocation has no input at `place`.
    pub(crate) fn input(
        &self,
        place: InputNumber,
    ) -> Result<RepresentationInput<'f>, OperatingError> {
        self.inputs
            .get(place.0)
            .copied()
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })
    }
    /// The rows of the input at `place`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] for a place beyond the three an operator reads,
    /// [`OperatingError::MissingInput`] when the invocation has no input there, and the refusal of admitting it.
    pub(crate) fn rows(
        &self,
        place: InputNumber,
    ) -> Result<RepresentationInput<'f>, OperatingError> {
        Ok(self.proof(place)?.rows())
    }
    pub(crate) fn proof(&self, place: InputNumber) -> Result<&BorrowedInput<'f>, OperatingError> {
        let cell = self
            .proofs
            .get(place.0)
            .ok_or(OperatingError::WrongOperation)?;
        if cell.get().is_none() {
            let rows = *self
                .inputs
                .get(place.0)
                .ok_or(OperatingError::MissingInput {
                    position: place.0 + 1,
                })?;
            cell.set(self.workspace.admit(place, rows)?)
                .map_err(|_| OperatingError::WrongOperation)?;
        }
        cell.get().ok_or(OperatingError::WrongOperation)
    }
}
