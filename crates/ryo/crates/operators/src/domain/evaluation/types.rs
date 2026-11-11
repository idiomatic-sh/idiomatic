//! One evaluation of a plan operation: the borrowed views the engine lends, the scratch it keeps for the
//! operators, and the driver port the engine implements once. Every kind's row meaning is written once, in
//! operators, over the part of the port its concept declares; the engine's physical strategies (index
//! probes, native candidates, walks over its trees and partitions) are implementations of those parts.
use crate::domain::axis::types::AxisEvaluation;
use crate::domain::contract::types::RowScratch;
use crate::domain::cycles::types::{CycleScratch, CyclesEvaluation};
use crate::domain::equality_join::types::JoinEvaluation;
use crate::domain::ordered_dfs::types::DfsEvaluation;
use crate::domain::ordered_match::types::OrderedMatchEvaluation;
use crate::domain::set_rows::types::SetRowsEvaluation;
use crate::domain::source_matches::types::{LineBytes, TextRowsEvaluation};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The rows one evaluation is given: one scheduled row of its primary input, or its complete populations,
/// which the operators read through the driver.
#[derive(Clone, Copy, Debug)]
pub enum EvaluationInput<'a> {
    /// One scheduled row.
    Row(&'a [Value]),
    /// The complete contributing populations, or the request's own source rows.
    Populations,
}

/// A resource the engine lends one evaluation: bound to it, or not bound, which an operation that reads it
/// refuses as a wrong binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lent<T> {
    /// The engine acquired the resource for this evaluation.
    Bound(T),
    /// No such resource is bound to this evaluation.
    Unbound,
}

impl<T> From<Option<T>> for Lent<T> {
    /// A present resource is `Bound`, an absent one `Unbound`.
    fn from(resource: Option<T>) -> Self {
        match resource {
            Some(bound) => Self::Bound(bound),
            None => Self::Unbound,
        }
    }
}

impl<T> From<Lent<T>> for Option<T> {
    /// A `Bound` resource is present, an `Unbound` one absent.
    fn from(resource: Lent<T>) -> Self {
        match resource {
            Lent::Bound(bound) => Some(bound),
            Lent::Unbound => None,
        }
    }
}

/// Plain data of one invocation, copied out of the engine's views before the driver borrows the engine.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationViews<'a> {
    /// The physical source of the file being read.
    pub source: LineBytes<'a>,
    /// The request's parameter bindings.
    pub bindings: &'a [ParameterBinding],
    /// The matcher the engine acquired for this operation.
    pub regex: Lent<&'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    /// The span text an occurrence search was validated against.
    pub checked: Lent<crate::domain::source_matches::types::CheckedSource<'a>>,
    /// The resolved suffix an extension appends.
    pub appended: Lent<&'a [Value]>,
    /// The resolved text-set layers of a rank.
    pub layers: Lent<&'a [Vec<LiteralText>]>,
    /// The complete reachability answer of a logical closure join.
    pub cached_reach: Lent<crate::domain::equality_join::types::CachedReach<'a>>,
    /// The complete answer of one axis request, after domain membership and nearest filtering.
    pub cached_axis: Lent<&'a [Value]>,
}

/// The engine's whole side of one evaluation: every part of the port a kind's concept declares.
pub trait InvocationDriver:
    JoinEvaluation
    + SetRowsEvaluation
    + TextRowsEvaluation
    + OrderedMatchEvaluation
    + CyclesEvaluation
    + AxisEvaluation
    + DfsEvaluation
{
}

impl<T> InvocationDriver for T where
    T: JoinEvaluation
        + SetRowsEvaluation
        + TextRowsEvaluation
        + OrderedMatchEvaluation
        + CyclesEvaluation
        + AxisEvaluation
        + DfsEvaluation
        + ?Sized
{
}

/// The buffers one evaluation fills, kept by the engine between invocations so they keep their capacity.
#[derive(Debug, Default)]
pub struct EvaluationScratch {
    /// The row buffers every kind reads.
    pub rows: RowScratch,
    /// The work arrays of a cycle search.
    pub cycles: CycleScratch,
}
