//! The `view` application operation: every view an invocation binds is acquired here, from its admitted inputs.
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
use ryo_execution_store::application::file_dag::workspace::service::InputAdmissions;
use ryo_execution_store::application::file_dag::workspace::types::ViewInputs;

/// A view the file's derived owner leased to one invocation: held while the invocation runs, and lent to primitives
/// only as its typed view.
#[derive(Debug)]
pub(crate) struct LeasedView(AcquiredView);

impl From<AcquiredView> for LeasedView {
    fn from(view: AcquiredView) -> Self {
        Self(view)
    }
}

impl LeasedView {
    /// The view `use_` plans over the admitted rows of its inputs.
    pub(crate) fn rows(
        admitted: &InputAdmissions<'_, '_>,
        use_: &PreparedViewUse,
        context: &mut OperatorContext<'_>,
    ) -> Result<Self, OperatingError> {
        admitted.acquire_rows(use_, context).map(Self)
    }
    /// The view `use_` plans over the inputs `supplied` names, under the view context `identity`.
    pub(crate) fn with_inputs(
        admitted: &InputAdmissions<'_, '_>,
        use_: &PreparedViewUse,
        identity: ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<Self, OperatingError> {
        admitted.acquire_derived(use_, identity, supplied).map(Self)
    }
    /// The native lookup `use_` plans, when its inputs admit one.
    pub(crate) fn native_lookup(
        admitted: &InputAdmissions<'_, '_>,
        use_: &PreparedViewUse,
    ) -> Result<Option<Self>, OperatingError> {
        Ok(admitted.try_native_lookup(use_)?.map(Self))
    }
    /// The closure partition the cursor stands on.
    pub(crate) fn current_partition(
        cursor: &mut PartitionedClosure<'_>,
    ) -> Result<Self, OperatingError> {
        cursor.take_current().map(Self)
    }
    /// The view the owner built for the lease's plan.
    pub(crate) fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        self.0.view()
    }
}
