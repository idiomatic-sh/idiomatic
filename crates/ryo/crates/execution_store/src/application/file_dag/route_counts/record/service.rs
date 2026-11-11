//! The route accounting's records: what each of its values states, which a format then writes.
use super::super::constants;
#[cfg(feature = "diagnostic-row-progress")]
use super::super::types::{RouteCardinality, RouteCounters, RouteHotRow};
use super::super::types::{
    RouteCountRow, RouteOwnerId, RouteOwnership, RoutePlanId, RoutePopulationId, RouteRuleName,
    RouteRuleRow, RouteTally,
};
use crate::application::file_dag::route_counts::ids::service::{
    RouteLabel, RouteNodeIndex, RouteRuleIndex,
};
use crate::outbound::route_counts::interface::RouteCountsPort;
use crate::outbound::route_counts::types::{
    RecordCount, RecordFlag, RecordKey, RecordTally, RecordText, RouteDocument, RouteField,
    RouteValue,
};

/// A value that states itself as a route record, which a format then writes.
pub(crate) trait Record {
    /// The value as a route record.
    fn record(&self) -> RouteValue;
}
/// The document `format` writes for `value`.
pub(crate) fn document(
    format: &dyn RouteCountsPort,
    value: &(impl Record + ?Sized),
) -> RouteDocument {
    format.document(&value.record())
}
/// One named field of a record.
pub(crate) fn field(key: RecordKey, value: &(impl Record + ?Sized)) -> RouteField {
    RouteField {
        key,
        value: value.record(),
    }
}

impl Record for RouteLabel {
    fn record(&self) -> RouteValue {
        RouteValue::Text(RecordText(self.0.to_string()))
    }
}

impl Record for RouteRuleName {
    fn record(&self) -> RouteValue {
        RouteValue::Text(RecordText(self.0.to_string()))
    }
}

impl Record for RouteRuleIndex {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RouteNodeIndex {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RouteOwnerId {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RoutePlanId {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RoutePopulationId {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RouteTally {
    fn record(&self) -> RouteValue {
        RouteValue::Tally(RecordTally(self.0))
    }
}

#[cfg(feature = "diagnostic-row-progress")]
impl Record for RouteCardinality {
    fn record(&self) -> RouteValue {
        RouteValue::Count(RecordCount(self.0))
    }
}

impl Record for RouteOwnership {
    fn record(&self) -> RouteValue {
        RouteValue::Flag(RecordFlag(*self == Self::Owned))
    }
}

impl<T: Record> Record for Option<T> {
    fn record(&self) -> RouteValue {
        self.as_ref().map_or(RouteValue::Absent, Record::record)
    }
}

impl<T: Record> Record for [T] {
    fn record(&self) -> RouteValue {
        RouteValue::List(self.iter().map(Record::record).collect())
    }
}

impl<T: Record, const N: usize> Record for [T; N] {
    fn record(&self) -> RouteValue {
        self.as_slice().record()
    }
}

impl<T: Record> Record for Vec<T> {
    fn record(&self) -> RouteValue {
        self.as_slice().record()
    }
}

impl Record for RouteCountRow {
    fn record(&self) -> RouteValue {
        RouteValue::Record(vec![
            field(constants::KEY_RULE, &self.rule),
            field(constants::KEY_NODE, &self.node),
            field(constants::KEY_OWNER, &self.owner),
            field(constants::KEY_OWNER_ID, &self.owner_id),
            field(constants::KEY_CONSUMER, &self.consumer),
            field(constants::KEY_PLAN, &self.plan),
            field(constants::KEY_POPULATIONS, &self.populations),
            field(constants::KEY_ROUTE, &self.route),
            field(constants::KEY_FAMILY, &self.family),
            field(constants::KEY_CALLS, &self.calls),
            field(constants::KEY_SUCCESSES, &self.successes),
            field(constants::KEY_FAILURES, &self.failures),
        ])
    }
}

impl Record for RouteRuleRow {
    fn record(&self) -> RouteValue {
        RouteValue::Record(vec![
            field(constants::KEY_INDEX, &self.index),
            field(constants::KEY_NAME, &self.name),
        ])
    }
}

#[cfg(feature = "diagnostic-row-progress")]
impl Record for RouteHotRow {
    fn record(&self) -> RouteValue {
        RouteValue::Record(vec![
            field(constants::KEY_RULE, &self.rule),
            field(constants::KEY_NODE, &self.node),
            field(constants::KEY_COUNTER, &self.counter),
            field(constants::KEY_COUNT, &self.count),
            field(constants::KEY_RULE_NAME, &self.rule_name),
            field(
                constants::KEY_INPUT_CARDINALITIES,
                &self.input_cardinalities,
            ),
            field(constants::KEY_NODE_NAMESPACE, &self.node_namespace),
        ])
    }
}

#[cfg(feature = "diagnostic-row-progress")]
impl Record for RouteCounters {
    fn record(&self) -> RouteValue {
        RouteValue::Record(vec![
            field(RecordKey(constants::COUNTER_INPUT_ROWS), &self.input_rows),
            field(
                RecordKey(constants::COUNTER_EMITTED_ROWS),
                &self.emitted_rows,
            ),
            field(
                RecordKey(constants::COUNTER_PREDICATE_EVALUATIONS),
                &self.predicate_evaluations,
            ),
            field(
                RecordKey(constants::COUNTER_INDEX_PROBES),
                &self.index_probes,
            ),
            field(
                RecordKey(constants::COUNTER_INDEX_ROWS_EXAMINED),
                &self.index_chain_rows_examined,
            ),
            field(
                RecordKey(constants::COUNTER_EQUALITY_MATCHES),
                &self.equality_matches,
            ),
            field(
                RecordKey(constants::COUNTER_JOIN_CANDIDATE_PAIRS),
                &self.join_candidate_pairs,
            ),
            field(
                RecordKey(constants::COUNTER_KERNEL_OUTPUT_ROWS),
                &self.kernel_output_rows,
            ),
            field(
                RecordKey(constants::COUNTER_POSITION_ROWS_YIELDED),
                &self.position_rows_yielded,
            ),
        ])
    }
}
