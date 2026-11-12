//! The views an invocation's admitted inputs demand: the workspace's inputs are admitted to the derived owner and the
//! view they name is acquired here, inside the derived concept, so the workspace hands out what the owner issued
//! without holding the owner's acquiring entries itself.
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::acquisition::population::types::StoredColumns;
use crate::application::file_dag::relations::admission::service::BorrowedInput;
use crate::application::file_dag::workspace::service::{
    GraphInput, InputAdmissions, InvocationWorkspace,
};
use crate::application::file_dag::workspace::types::ViewInputs;
use ryo_execution_domain::domain::derived_views::types::InputsUsed;
use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

impl InvocationWorkspace {
    pub(crate) fn acquire_admitted(
        &self,
        use_: &PreparedViewUse,
        inputs: &InputAdmissions<'_, '_>,
        identity: crate::application::file_dag::derived::owner::service::ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<AcquiredView, OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        use ryo_execution_domain::domain::derived_views::types::{DerivedInputs, MembershipSpace};
        let count = usize::from(use_.len.0);
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
        // The prepared Membership slot also declares the native tree space.
        // Keep admission/error order for all demanded operands, while the value
        // view's identity and row proof contain only its actual semantic source.
        let semantic_sources = if matches!(&supplied, ViewInputs::MembershipValues) {
            1
        } else {
            count
        };
        let mut proofs: [Option<&BorrowedInput<'_>>; 2] = [None, None];
        let mut relations = [None; 2];
        for at in 0..count {
            let proof = inputs.proof(use_.inputs[at])?;
            let relation = proof.identity(self.owner())?.relation;
            relations[at] = relation;
            proofs[at] = Some(proof);
        }
        let Some(first) = proofs[0] else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = [
            first.rows(),
            proofs[1].as_ref().map_or(
                RepresentationInput::Computed(&EMPTY_DERIVED_INPUT),
                |proof| proof.rows(),
            ),
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
                let RepresentationInput::Tree(tree) = rows[1] else {
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
                RepresentationInput::Stored(source) => relations[0]
                    .or(use_.relations[0])
                    .map(|relation| StoredColumns { relation }),
                RepresentationInput::Tree(_)
                | RepresentationInput::Computed(_)
                | RepresentationInput::Compact(_) => None,
            }
        } else {
            None
        };
        let owner = self.owner().derived();
        let sources = proofs[..semantic_sources]
            .iter()
            .map(|proof| proof.ok_or(OperatingError::WrongOperation))
            .collect::<Result<Vec<_>, _>>()?;
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

impl<'a> GraphInput<'a> {
    pub(crate) fn admission(
        &self,
        plan: &Arc<ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract>,
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AdmittedViewInputs<'a>,
        OperatingError,
    >{
        self.owner
            .derived()
            .admit_view(plan, self.owner, &[&self.proof])
    }
}

impl<'f> InputAdmissions<'_, 'f> {
    /// The native lookup view of the tree input, when the input is a tree.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] when `use_` is no native lookup over two inputs, and the refusals of admitting
    /// its inputs and acquiring the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// fn native(admissions: &InputAdmissions<'_, '_>, use_: &PreparedViewUse) -> bool {
    ///     matches!(admissions.try_native_lookup(use_), Ok(Some(_)))
    /// }
    /// ```
    pub fn try_native_lookup(
        &self,
        use_: &PreparedViewUse,
    ) -> Result<Option<AcquiredView>, OperatingError> {
        if use_.plan.kind != ryo_derived::domain::request::types::DerivedKind::NativeLookup
            || use_.len != InputsUsed(2)
        {
            return Err(OperatingError::WrongOperation);
        }
        let left = self.proof(use_.inputs[0])?;
        let right = self.proof(use_.inputs[1])?;
        let owner = self.workspace.owner();
        let admitted = owner
            .derived()
            .admit_view(&use_.plan, owner, &[left, right])?;
        owner.derived().try_acquire_rows(
            &use_.plan,
            &admitted,
            ryo_execution_domain::domain::derived_views::types::DerivedInputs::Rows(&[
                left.rows(),
                right.rows(),
            ]),
            &mut self.workspace.derived_scratch.borrow_mut(),
        )
    }
    /// Acquires the row view `use_` names.
    ///
    /// # Errors
    ///
    /// The refusals of [`Self::acquire_derived`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// use ryo_execution_domain::domain::operator::types::OperatorContext;
    /// fn acquired(admissions: &InputAdmissions<'_, '_>, use_: &PreparedViewUse, context: &mut OperatorContext<'_>) -> bool {
    ///     admissions.acquire_rows(use_, context).is_ok()
    /// }
    /// ```
    pub fn acquire_rows(
        &self,
        use_: &PreparedViewUse,
        context: &mut OperatorContext<'_>,
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_derived(
            use_,
            crate::application::file_dag::derived::owner::service::ViewContext::default(),
            ViewInputs::Rows,
        )
    }
    /// Acquires the derived view `use_` names over `supplied`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] when `use_` reads no input or more than two, or its inputs do not fit
    /// `supplied`, [`OperatingError::MissingInput`] when `supplied` needs a second input `use_` lacks, and the refusals of
    /// admitting its inputs and acquiring the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
    /// use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
    /// use ryo_execution_store::application::file_dag::workspace::types::ViewInputs;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// fn acquired(admissions: &InputAdmissions<'_, '_>, use_: &PreparedViewUse, supplied: ViewInputs<'_, '_>) -> bool {
    ///     admissions.acquire_derived(use_, ViewContext::default(), supplied).is_ok()
    /// }
    /// ```
    pub fn acquire_derived(
        &self,
        use_: &PreparedViewUse,
        identity: crate::application::file_dag::derived::owner::service::ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<AcquiredView, OperatingError> {
        self.workspace
            .acquire_admitted(use_, self, identity, supplied)
    }
    /// The partitioned closure of `input`, built from its shared ordering.
    ///
    /// # Errors
    ///
    /// The refusal of admitting the ordering's input, [`OperatingError::NotRead`] when `input` is not the rows it
    /// admitted, and the refusals of acquiring the ordering and preparing the closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
    /// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// use ryo_execution_domain::domain::operator::types::OperatorContext;
    /// use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    /// fn partitions<'f>(
    ///     admissions: &InputAdmissions<'_, 'f>,
    ///     order: &PreparedViewUse,
    ///     closure: &PreparedViewUse,
    ///     input: RepresentationInput<'f>,
    ///     context: &mut OperatorContext<'_>,
    /// ) -> Option<PartitionedClosure<'f>> {
    ///     admissions.partitioned(order, closure, input, context).ok()
    /// }
    /// ```
    pub fn partitioned(
        &self,
        order: &PreparedViewUse,
        closure: &PreparedViewUse,
        input: RepresentationInput<'f>,
        context: &mut OperatorContext<'_>,
    ) -> Result<
        crate::application::file_dag::closure_partitions::service::PartitionedClosure<'f>,
        OperatingError,
    > {
        let proof = self.proof(order.inputs[0])?;
        if crate::application::file_dag::relations::service::same_admitted_rows(input, proof.rows())
            == crate::application::file_dag::relations::types::Sameness::Different
        {
            return Err(OperatingError::NotRead);
        }
        let storage = self.acquire_rows(order, context)?;
        crate::application::file_dag::derived::partition_views::service::from_prepared(
            proof.clone(),
            Arc::clone(&order.plan),
            Arc::clone(&closure.plan),
            storage,
            self.owner_handle(),
        )
    }
}
