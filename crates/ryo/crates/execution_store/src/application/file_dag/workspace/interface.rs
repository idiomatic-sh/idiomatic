//! What the runtime needs from an invocation's workspace: the one pub route to its inputs, counters and working allocations.
use super::service::InputAdmissions;
use super::types::WorkspaceParts;
use crate::application::file_dag::relations::handle::service::Publication;
use ryo_execution_domain::domain::operator::contract::types::{PrimitiveWorkspace, WorkCounters};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;

/// Seals [`WorkspacePort`]: the trait is public so the runtime can reach the workspace through it, and its one
/// implementation is [`InvocationWorkspace`](super::service::InvocationWorkspace)'s.
pub(crate) mod sealed {
    /// Implemented only by `InvocationWorkspace`: no other type, in this crate or another, implements it or `WorkspacePort`.
    #[expect(
        unnameable_types,
        reason = "the seal: a supertrait other crates can reach but never name, so they cannot implement WorkspacePort"
    )]
    pub trait Sealed {}
}

/// What the runtime needs from an invocation's workspace: the one pub route to its inputs, counters and working
/// allocations. It is sealed: a type of another crate that answers every operation is still no workspace,
/// ```compile_fail,E0277
/// use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
/// use ryo_execution_domain::domain::operator::contract::types::{PrimitiveWorkspace, WorkCounters};
/// use ryo_execution_domain::domain::table_model::types::RepresentationInput;
/// use ryo_execution_store::application::file_dag::relations::handle::service::Publication;
/// use ryo_execution_store::application::file_dag::workspace::{
///     interface::WorkspacePort, service::InputAdmissions, types::WorkspaceParts,
/// };
/// struct Elsewhere;
/// impl WorkspacePort for Elsewhere {
///     fn inputs<'a, 'f>(&'a self, _: &'a [RepresentationInput<'f>]) -> InputAdmissions<'a, 'f> { todo!() }
///     fn work(&self) -> &WorkCounters { todo!() }
///     fn primitive(&self) -> &PrimitiveWorkspace { todo!() }
///     fn primitive_mut(&mut self) -> &mut PrimitiveWorkspace { todo!() }
///     fn parts(&mut self) -> WorkspaceParts<'_> { todo!() }
///     fn capture_admitted(&mut self, _: Vec<Publication>) {}
///     fn release_inputs(&mut self) {}
///     fn measured(&self) -> ExecutionObservation { todo!() }
/// }
/// ```
/// so the workspace the runtime holds is always the store's.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
/// use ryo_execution_store::application::file_dag::relations::handle::service::Publication;
/// fn invoked(workspace: &mut impl WorkspacePort, publications: Vec<Publication>) {
///     workspace.capture_admitted(publications);
///     workspace.release_inputs();
/// }
/// ```
pub trait WorkspacePort: sealed::Sealed {
    /// The admissions of `inputs` for this invocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    /// use ryo_values::domain::plan_parts::types::InputNumber;
    /// fn first<'f>(workspace: &impl WorkspacePort, inputs: &[RepresentationInput<'f>]) -> bool {
    ///     workspace.inputs(inputs).input(InputNumber(0)).is_ok()
    /// }
    /// ```
    fn inputs<'a, 'f>(&'a self, inputs: &'a [RepresentationInput<'f>]) -> InputAdmissions<'a, 'f>;
    /// The rows each input was found to hold, counted as the kernels report them.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// use ryo_execution_domain::domain::operator::contract::lease::service::take_totals;
    /// use ryo_execution_domain::domain::operator::contract::types::WorkTotals;
    /// fn found(workspace: &impl WorkspacePort) -> WorkTotals {
    ///     take_totals(workspace.work())
    /// }
    /// ```
    fn work(&self) -> &WorkCounters;
    /// The primitive operators' working allocations, to read what they hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// fn positions(workspace: &impl WorkspacePort) -> usize {
    ///     workspace.primitive().positions.len()
    /// }
    /// ```
    fn primitive(&self) -> &PrimitiveWorkspace;
    /// The primitive operators' working allocations, lent to a kernel.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// fn cleared(workspace: &mut impl WorkspacePort) {
    ///     workspace.primitive_mut().positions.clear();
    /// }
    /// ```
    fn primitive_mut(&mut self) -> &mut PrimitiveWorkspace;
    /// Every working allocation a kernel uses, each lent apart from the others so one evaluation can hold them
    /// at once; the admitted publications stay with the workspace.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// fn positions(workspace: &mut impl WorkspacePort) -> usize {
    ///     workspace.parts().primitive.positions.len()
    /// }
    /// ```
    fn parts(&mut self) -> WorkspaceParts<'_>;
    /// Holds `publications` as the invocation's admitted inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// use ryo_execution_store::application::file_dag::relations::handle::service::Publication;
    /// fn admitted(workspace: &mut impl WorkspacePort, publications: Vec<Publication>) {
    ///     workspace.capture_admitted(publications);
    /// }
    /// ```
    fn capture_admitted(
        &mut self,
        publications: crate::application::file_dag::workspace::types::AdmittedPublications,
    );
    /// Releases the admitted publications.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// use ryo_execution_store::application::file_dag::relations::handle::service::Publication;
    /// fn invoked(workspace: &mut impl WorkspacePort, publications: Vec<Publication>) {
    ///     workspace.capture_admitted(publications);
    ///     workspace.release_inputs();
    /// }
    /// ```
    fn release_inputs(&mut self);
    /// The route profile position this invocation was opened at.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// use ryo_execution_store::application::file_dag::route_counts::profile::service::OriginScope;
    /// fn resumed(workspace: &impl WorkspacePort) -> OriginScope {
    ///     workspace.progress_origin().resume()
    /// }
    /// ```
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress_origin(
        &self,
    ) -> &crate::application::file_dag::route_counts::profile::service::CapturedOrigin;
    /// What the invocation's allocations measure now, for the proofs of their reuse and release.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
    /// fn rows_held(workspace: &impl WorkspacePort) -> usize {
    ///     workspace.measured().row_capacity
    /// }
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    fn measured(&self) -> ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
}
