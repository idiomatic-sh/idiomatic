//! What the runtime needs from an invocation's admitted inputs: their rows and the derived views they demand, the one pub
//! route to them.
use crate::application::file_dag::closure_partitions::service::PartitionedClosure;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::cached_graph::service::CachedGraph;
use crate::application::file_dag::derived::owner::service::ViewContext;
use crate::application::file_dag::workspace::types::ViewInputs;
use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::resources::types::ResourceProgram;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::table_model::types::{RepresentationInput, Rows};
use ryo_values::domain::plan_parts::types::InputNumber;

/// Seals [`DerivedAdmittedViewsPort`]: the trait is public so the runtime can reach the admissions through it, and its one
/// implementation is [`InputAdmissions`](crate::application::file_dag::workspace::service::InputAdmissions)'s.
pub(crate) mod sealed {
    /// Implemented only by `InputAdmissions`: no other type, in this crate or another, implements it or
    /// `DerivedAdmittedViewsPort`.
    #[expect(
        unnameable_types,
        reason = "the seal: a supertrait other crates can reach but never name, so they cannot implement DerivedAdmittedViewsPort"
    )]
    pub trait Sealed {}
}

/// What the runtime needs from an invocation's admitted inputs: their rows and the derived views they demand, the one pub
/// route to them. It is sealed: a type of another crate that answers every operation is still no admission,
/// ```compile_fail,E0277
/// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
/// use ryo_execution_domain::domain::operating::errors::OperatingError;
/// use ryo_execution_domain::domain::operator::contract::resources::types::ResourceProgram;
/// use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
/// use ryo_execution_domain::domain::operator::types::OperatorContext;
/// use ryo_execution_domain::domain::table_model::types::{RepresentationInput, Rows};
/// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
/// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
/// use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
/// use ryo_execution_store::application::file_dag::derived::owner::cached_graph::service::CachedGraph;
/// use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
/// use ryo_execution_store::application::file_dag::workspace::types::ViewInputs;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// struct Elsewhere;
/// impl<'f> DerivedAdmittedViewsPort<'f> for Elsewhere {
///     fn input(&self, _: InputNumber) -> Result<RepresentationInput<'f>, OperatingError> { todo!() }
///     fn rows(&self, _: InputNumber) -> Result<RepresentationInput<'f>, OperatingError> { todo!() }
///     fn try_native_lookup(&self, _: &PreparedViewUse) -> Result<Option<AcquiredView>, OperatingError> { todo!() }
///     fn acquire_rows(&self, _: &PreparedViewUse, _: &mut OperatorContext<'_>) -> Result<AcquiredView, OperatingError> {
///         todo!()
///     }
///     fn acquire_derived(
///         &self,
///         _: &PreparedViewUse,
///         _: ViewContext,
///         _: ViewInputs<'_, '_>,
///     ) -> Result<AcquiredView, OperatingError> { todo!() }
///     fn partitioned(
///         &self,
///         _: &PreparedViewUse,
///         _: &PreparedViewUse,
///         _: RepresentationInput<'f>,
///         _: &mut OperatorContext<'_>,
///     ) -> Result<PartitionedClosure<'f>, OperatingError> { todo!() }
///     fn prepare_graph(
///         &self,
///         _: &OperatorContract,
///         _: &ResourceProgram,
///         _: &[Rows<'_>],
///     ) -> Result<Option<CachedGraph>, OperatingError> { todo!() }
/// }
/// ```
/// so the admissions the runtime reads are always the workspace's.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// fn held<'f>(admissions: &impl DerivedAdmittedViewsPort<'f>) -> bool {
///     admissions.input(InputNumber(0)).is_ok() && admissions.rows(InputNumber(0)).is_ok()
/// }
/// ```
pub trait DerivedAdmittedViewsPort<'f>: sealed::Sealed {
    /// The input at `place`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::MissingInput`] when the invocation has no input at `place`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_values::domain::plan_parts::types::InputNumber;
    /// fn admitted<'f>(admissions: &impl DerivedAdmittedViewsPort<'f>) -> bool {
    ///     admissions.input(InputNumber(0)).is_ok()
    /// }
    /// ```
    fn input(&self, place: InputNumber) -> Result<RepresentationInput<'f>, OperatingError>;
    /// The rows of the input at `place`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] for a place beyond the three an operator reads,
    /// [`OperatingError::MissingInput`] when the invocation has no input there, and the refusal of admitting it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_values::domain::plan_parts::types::InputNumber;
    /// fn held<'f>(admissions: &impl DerivedAdmittedViewsPort<'f>) -> bool {
    ///     admissions.rows(InputNumber(0)).is_ok()
    /// }
    /// ```
    fn rows(&self, place: InputNumber) -> Result<RepresentationInput<'f>, OperatingError>;
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
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// fn native<'f>(admissions: &impl DerivedAdmittedViewsPort<'f>, use_: &PreparedViewUse) -> bool {
    ///     matches!(admissions.try_native_lookup(use_), Ok(ryo_execution_store::application::file_dag::relations::types::Sought::Found(_)))
    /// }
    /// ```
    fn try_native_lookup(
        &self,
        use_: &PreparedViewUse,
    ) -> Result<crate::application::file_dag::relations::types::Sought<AcquiredView>, OperatingError>;
    /// Acquires the row view `use_` names.
    ///
    /// # Errors
    ///
    /// The refusals of [`Self::acquire_derived`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// use ryo_execution_domain::domain::operator::types::OperatorContext;
    /// fn acquired<'f>(
    ///     admissions: &impl DerivedAdmittedViewsPort<'f>,
    ///     use_: &PreparedViewUse,
    ///     context: &mut OperatorContext<'_>,
    /// ) -> bool {
    ///     admissions.acquire_rows(use_, context).is_ok()
    /// }
    /// ```
    fn acquire_rows(
        &self,
        use_: &PreparedViewUse,
        context: &mut OperatorContext<'_>,
    ) -> Result<AcquiredView, OperatingError>;
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
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
    /// use ryo_execution_store::application::file_dag::workspace::types::ViewInputs;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// fn acquired<'f>(
    ///     admissions: &impl DerivedAdmittedViewsPort<'f>,
    ///     use_: &PreparedViewUse,
    ///     supplied: ViewInputs<'_, '_>,
    /// ) -> bool {
    ///     admissions.acquire_derived(use_, ViewContext::default(), supplied).is_ok()
    /// }
    /// ```
    fn acquire_derived(
        &self,
        use_: &PreparedViewUse,
        identity: ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<AcquiredView, OperatingError>;
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
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
    /// use ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse;
    /// use ryo_execution_domain::domain::operator::types::OperatorContext;
    /// use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    /// fn partitions<'f>(
    ///     admissions: &impl DerivedAdmittedViewsPort<'f>,
    ///     order: &PreparedViewUse,
    ///     closure: &PreparedViewUse,
    ///     input: RepresentationInput<'f>,
    ///     context: &mut OperatorContext<'_>,
    /// ) -> Option<PartitionedClosure<'f>> {
    ///     admissions.partitioned(order, closure, input, context).ok()
    /// }
    /// ```
    fn partitioned(
        &self,
        order: &PreparedViewUse,
        closure: &PreparedViewUse,
        input: RepresentationInput<'f>,
        context: &mut OperatorContext<'_>,
    ) -> Result<PartitionedClosure<'f>, OperatingError>;
    /// Prepares the cached graph an axis or reach operator reads over `rows`, the original rows of the admitted inputs,
    /// when its program caches one.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the file's graph cache is not prepared, and the refusals of admitting the graph's
    /// inputs and acquiring the graph and its domain.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_store::application::file_dag::derived::owner::cached_graph::service::CachedGraph;
    /// use ryo_execution_domain::domain::operator::contract::resources::types::ResourceProgram;
    /// use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
    /// use ryo_execution_domain::domain::table_model::types::Rows;
    /// fn graph<'f>(
    ///     admissions: &impl DerivedAdmittedViewsPort<'f>,
    ///     contract: &OperatorContract,
    ///     program: &ResourceProgram,
    ///     rows: &[Rows<'_>],
    /// ) -> Option<CachedGraph> {
    ///     admissions.prepare_graph(contract, program, rows).ok().and_then(Option::from)
    /// }
    /// ```
    fn prepare_graph(
        &self,
        contract: &OperatorContract,
        program: &ResourceProgram,
        rows: &[Rows<'_>],
    ) -> Result<crate::application::file_dag::relations::types::Sought<CachedGraph>, OperatingError>;
}
