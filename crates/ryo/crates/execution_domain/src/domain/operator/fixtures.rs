//! The `operator` concept's test support: a primitive lease over fakes of the invocation's ports, for the
//! cases of the values behind the operators' driver ports.

use super::contract::lease::service::primitive_lease;
use super::contract::types::{
    PrimitiveLease, PrimitiveResources, PrimitiveViews, PrimitiveWorkspace, RowSink, WorkCounters,
};
use super::contract::views::types::{ClosureViews, NativeJoin, PredicateProgram, PreparedAxis};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{AfterTheFiles, PrimitiveSemantics};
use ryo_operators::domain::contract::types::PredicateOutcome;
use crate::domain::table_reach::borrowed::types::IndexedRows;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::SourceView;

/// Resources with nothing acquired but, when one is given, the axis.
#[derive(Default)]
pub(crate) struct Resources {
    /// The axis the operation walks, when it walks one.
    pub(crate) axis: Option<PreparedAxis<'static>>,
}

impl PrimitiveResources for Resources {
    fn validate_declared(&self) -> Result<(), OperatingError> {
        Ok(())
    }
    fn index(&self) -> Option<IndexedRows<'_>> {
        None
    }
    fn appended(&self) -> Option<&[Value]> {
        None
    }
    fn rank_layers(&self) -> Option<&[Vec<ryo_values::domain::plan_parts::types::LiteralText>]> {
        None
    }
    fn axis(&self) -> Result<Option<PreparedAxis<'_>>, OperatingError> {
        Ok(self.axis)
    }
    fn native_join(&self) -> Option<NativeJoin<'_>> {
        None
    }
    fn dfs(&self) -> Result<Option<&ryo_derived::domain::dfs::types::PreparedDfs>, OperatingError> {
        Ok(None)
    }
    fn cycles(
        &self,
    ) -> Result<Option<&ryo_derived::domain::cycle_adjacency::types::CycleGraph>, OperatingError>
    {
        Ok(None)
    }
    fn closure(&self) -> Result<ClosureViews<'_>, OperatingError> {
        Ok(ClosureViews::Empty)
    }
    fn ordered(
        &self,
    ) -> Result<Option<&ryo_derived::domain::ordered::types::OrderedMatching>, OperatingError> {
        Ok(None)
    }
    fn groups(&self) -> Option<&[crate::domain::table_groups::types::Group]> {
        None
    }
}

/// A predicate that answers one verdict for every row.
pub(crate) struct Judge(pub(crate) PredicateOutcome);

impl PredicateProgram for Judge {
    fn holds(&mut self, _row: &[Value], _symbols: &Symbols) -> Result<PredicateOutcome, OperatingError> {
        Ok(self.0)
    }
}

/// Semantics whose operators are never called by the values' cases.
pub(crate) struct Uncalled;

impl PrimitiveSemantics for Uncalled {
    fn operators(&self, _call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        Err(PortableOperatorFault::WrongBinding)
    }
}

/// A sink that keeps every row delivered to it.
#[derive(Default)]
pub(crate) struct Kept(pub(crate) Vec<Vec<Value>>);

impl RowSink for Kept {
    fn row(
        &mut self,
        _lease: &mut PrimitiveLease<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        self.0.push(row.to_vec());
        Ok(())
    }
}

/// Runs `body` with a lease over `resources` and `workspace`, whose predicate answers `verdict`.
pub(crate) fn with_lease<R>(
    resources: &dyn PrimitiveResources,
    verdict: PredicateOutcome,
    workspace: &mut PrimitiveWorkspace,
    body: impl FnOnce(&mut PrimitiveLease<'_>) -> R,
) -> R {
    let mut symbols = Symbols::default();
    let mut predicate = Judge(verdict);
    let counters = WorkCounters::default();
    let views = PrimitiveViews {
        phase: AfterTheFiles::Before,
        resources,
        semantic: None,
        paths: &[],
        arenas: &[],
        source: SourceView(b""),
        bindings: &[],
        validated_source: None,
        regex: None,
        cached_reach: None,
        cached_axis: None,
    };
    let mut lease = primitive_lease(
        views,
        workspace,
        QuerySymbols::from(&mut symbols),
        &mut predicate,
        &Uncalled,
        &counters,
    );
    body(&mut lease)
}

/// An operation of no strategy's kind: it extends an earlier step by no column.
pub(crate) fn extend() -> ryo_values::domain::plans::types::Operation {
    ryo_values::domain::plans::types::Operation::Extend {
        from: ryo_values::domain::plan_parts::types::Input::Step(
            ryo_values::domain::plan_parts::types::StepId(0),
        ),
        values: Vec::new(),
    }
}
