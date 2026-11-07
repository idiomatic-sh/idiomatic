//! The mandatory exhaustive executable registry. No optional or residual path exists.
use crate::domain::operator::collections::operators::antijoin::item::AntijoinItem;
use crate::domain::operator::collections::operators::axis::item::AxisItem;
use crate::domain::operator::collections::operators::capture::item::CaptureItem;
use crate::domain::operator::collections::operators::closure::item::ClosureItem;
use crate::domain::operator::collections::operators::cycles::item::CyclesItem;
use crate::domain::operator::collections::operators::difference::item::DifferenceItem;
use crate::domain::operator::collections::operators::extend::item::ExtendItem;
use crate::domain::operator::collections::operators::fits_rendered::item::FitsRenderedItem;
use crate::domain::operator::collections::operators::group_count::item::GroupCountItem;
use crate::domain::operator::collections::operators::group_digest::item::GroupDigestItem;
use crate::domain::operator::collections::operators::hosted::item::HostedItem;
use crate::domain::operator::collections::operators::interface::OperatorsItem;
use crate::domain::operator::collections::operators::intersection::item::IntersectionItem;
use crate::domain::operator::collections::operators::join::item::JoinItem;
use crate::domain::operator::collections::operators::ordered_dfs::item::OrderedDfsItem;
use crate::domain::operator::collections::operators::ordered_match::item::OrderedMatchItem;
use crate::domain::operator::collections::operators::project::item::ProjectItem;
use crate::domain::operator::collections::operators::rank::item::RankItem;
use crate::domain::operator::collections::operators::render_syntax::item::RenderSyntaxItem;
use crate::domain::operator::collections::operators::replace::item::ReplaceItem;
use crate::domain::operator::collections::operators::rows::item::RowsItem;
use crate::domain::operator::collections::operators::select::item::SelectItem;
use crate::domain::operator::collections::operators::source_matches::item::SourceMatchesItem;
use crate::domain::operator::collections::operators::span_between::item::SpanBetweenItem;
use crate::domain::operator::collections::operators::span_bounds::item::SpanBoundsItem;
use crate::domain::operator::collections::operators::union::item::UnionItem;
use crate::domain::operator::collections::operators::words::item::WordsItem;
use ryo_values::domain::query_operators::types::OperationKind;
/// Only this registry constructs a callable operator selected by the exhaustive kind.
/// The implementation trait stays inside the operator concept.
pub(crate) struct RegisteredOperator {
    kind: OperationKind,
}
impl RegisteredOperator {
    pub(in crate::domain::operator) fn prepare(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    ) -> Result<super::types::OperatorContract, crate::domain::operating::errors::OperatingError>
    {
        if ryo_values::domain::plans::service::kind_of(operation) != self.kind {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        self.item().prepare(operation, inputs, bindings)
    }
    pub(crate) fn evaluate(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
        inputs: super::types::PreparedInputs<'_>,
        lease: &mut super::types::PrimitiveLease<'_>,
        output: &mut dyn super::types::RowSink,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        if ryo_values::domain::plans::service::kind_of(operation) != self.kind {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        self.item().evaluate(operation, inputs, lease, output)
    }
}
impl RegisteredOperator {
    fn item(&self) -> &'static dyn OperatorsItem {
        match self.kind {
            OperationKind::Replace => &ReplaceItem,
            OperationKind::OrderedMatch => &OrderedMatchItem,
            OperationKind::Select => &SelectItem,
            OperationKind::Project => &ProjectItem,
            OperationKind::Extend => &ExtendItem,
            OperationKind::Join => &JoinItem,
            OperationKind::Union => &UnionItem,
            OperationKind::Intersection => &IntersectionItem,
            OperationKind::Difference => &DifferenceItem,
            OperationKind::Antijoin => &AntijoinItem,
            OperationKind::GroupCount => &GroupCountItem,
            OperationKind::GroupDigest => &GroupDigestItem,
            OperationKind::Capture => &CaptureItem,
            OperationKind::Rank => &RankItem,
            OperationKind::Words => &WordsItem,
            OperationKind::Axis => &AxisItem,
            OperationKind::Closure => &ClosureItem,
            OperationKind::Cycles => &CyclesItem,
            OperationKind::Rows => &RowsItem,
            OperationKind::Hosted => &HostedItem,
            OperationKind::SpanBetween => &SpanBetweenItem,
            OperationKind::SourceMatches => &SourceMatchesItem,
            OperationKind::RenderSyntax => &RenderSyntaxItem,
            OperationKind::FitsRendered => &FitsRenderedItem,
            OperationKind::SpanBounds => &SpanBoundsItem,
            OperationKind::OrderedDfs => &OrderedDfsItem,
        }
    }
}
pub(crate) fn registered(kind: OperationKind) -> RegisteredOperator {
    RegisteredOperator { kind }
}
