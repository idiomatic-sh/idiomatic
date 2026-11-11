//! JSON shapes of the route accounting events, written field by field.
use super::constants;
#[cfg(feature = "diagnostic-row-progress")]
use super::types::RouteCardinality;
use super::types::{
    RouteCountRow, RouteDocument, RouteOwnerId, RouteOwnership, RoutePlanId, RoutePopulationId,
    RouteRuleName, RouteRuleRow, RouteTally,
};
#[cfg(feature = "diagnostic-row-progress")]
use super::types::{RouteCounters, RouteHotRow};
use crate::application::file_dag::route_counts::ids::service::RouteLabel;
use crate::application::file_dag::route_counts::ids::service::RouteNodeIndex;
use crate::application::file_dag::route_counts::ids::service::RouteRuleIndex;

/// A value that writes itself as JSON.
pub(super) trait Encode {
    fn encode(&self, out: &mut RouteDocument);
}
/// Whether the next field of an object needs a separator.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    First,
    Rest,
}
pub(super) struct Object<'a> {
    out: &'a mut RouteDocument,
    position: Position,
}
pub(super) fn document(value: &impl Encode) -> RouteDocument {
    let mut out = RouteDocument::default();
    value.encode(&mut out);
    out
}
pub(super) fn object(out: &mut RouteDocument, build: impl FnOnce(&mut Object<'_>)) {
    out.0.push(constants::OPEN_OBJECT);
    build(&mut Object {
        out: &mut *out,
        position: Position::First,
    });
    out.0.push(constants::CLOSE_OBJECT);
}
impl Object<'_> {
    pub(super) fn field(&mut self, key: impl Into<RouteLabel>, value: &impl Encode) {
        if self.position == Position::Rest {
            self.out.0.push(constants::SEPARATOR);
        }
        self.position = Position::Rest;
        key.into().encode(self.out);
        self.out.0.push(constants::ASSIGN);
        value.encode(self.out);
    }
}
fn quote(out: &mut RouteDocument, text: impl AsRef<str>) {
    out.0.push(constants::QUOTE);
    for character in text.as_ref().chars() {
        match character {
            constants::QUOTE => out.0.push_str(constants::ESCAPED_QUOTE),
            constants::BACKSLASH => out.0.push_str(constants::ESCAPED_BACKSLASH),
            constants::NEWLINE => out.0.push_str(constants::ESCAPED_NEWLINE),
            constants::RETURN => out.0.push_str(constants::ESCAPED_RETURN),
            constants::TAB => out.0.push_str(constants::ESCAPED_TAB),
            control if control.is_control() => {
                out.0.push_str(constants::ESCAPE_UNICODE);
                for nibble in (0..constants::HEX_DIGITS).rev() {
                    let digit = (u32::from(control) >> (nibble * constants::HEX_NIBBLE_BITS))
                        & constants::HEX_NIBBLE_MASK;
                    out.0.extend(char::from_digit(digit, constants::HEX_RADIX));
                }
            }
            other => out.0.push(other),
        }
    }
    out.0.push(constants::QUOTE);
}
fn number(out: &mut RouteDocument, value: impl ToString) {
    out.0.push_str(&value.to_string());
}
impl Encode for RouteLabel {
    fn encode(&self, out: &mut RouteDocument) {
        quote(out, self.0);
    }
}
impl Encode for RouteRuleName {
    fn encode(&self, out: &mut RouteDocument) {
        quote(out, &self.0);
    }
}
impl Encode for RouteRuleIndex {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RouteNodeIndex {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RouteOwnerId {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RoutePlanId {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RoutePopulationId {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RouteTally {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
#[cfg(feature = "diagnostic-row-progress")]
impl Encode for RouteCardinality {
    fn encode(&self, out: &mut RouteDocument) {
        number(out, self.0);
    }
}
impl Encode for RouteOwnership {
    fn encode(&self, out: &mut RouteDocument) {
        out.0.push_str(if *self == Self::Owned {
            constants::TRUE
        } else {
            constants::FALSE
        });
    }
}
impl<T: Encode> Encode for Option<T> {
    fn encode(&self, out: &mut RouteDocument) {
        match self {
            Some(value) => value.encode(out),
            None => out.0.push_str(constants::NULL),
        }
    }
}
impl<T: Encode> Encode for [T] {
    fn encode(&self, out: &mut RouteDocument) {
        out.0.push(constants::OPEN_ARRAY);
        for (position, item) in self.iter().enumerate() {
            if position > 0 {
                out.0.push(constants::SEPARATOR);
            }
            item.encode(out);
        }
        out.0.push(constants::CLOSE_ARRAY);
    }
}
impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, out: &mut RouteDocument) {
        self.as_slice().encode(out);
    }
}
impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, out: &mut RouteDocument) {
        self.as_slice().encode(out);
    }
}

impl Encode for RouteCountRow {
    fn encode(&self, out: &mut RouteDocument) {
        object(out, |row| {
            row.field(constants::KEY_RULE, &self.rule);
            row.field(constants::KEY_NODE, &self.node);
            row.field(constants::KEY_OWNER, &self.owner);
            row.field(constants::KEY_OWNER_ID, &self.owner_id);
            row.field(constants::KEY_CONSUMER, &self.consumer);
            row.field(constants::KEY_PLAN, &self.plan);
            row.field(constants::KEY_POPULATIONS, &self.populations);
            row.field(constants::KEY_ROUTE, &self.route);
            row.field(constants::KEY_FAMILY, &self.family);
            row.field(constants::KEY_CALLS, &self.calls);
            row.field(constants::KEY_SUCCESSES, &self.successes);
            row.field(constants::KEY_FAILURES, &self.failures);
        });
    }
}
impl Encode for RouteRuleRow {
    fn encode(&self, out: &mut RouteDocument) {
        object(out, |row| {
            row.field(constants::KEY_INDEX, &self.index);
            row.field(constants::KEY_NAME, &self.name);
        });
    }
}
#[cfg(feature = "diagnostic-row-progress")]
impl Encode for RouteHotRow {
    fn encode(&self, out: &mut RouteDocument) {
        object(out, |row| {
            row.field(constants::KEY_RULE, &self.rule);
            row.field(constants::KEY_NODE, &self.node);
            row.field(constants::KEY_COUNTER, &self.counter);
            row.field(constants::KEY_COUNT, &self.count);
            row.field(constants::KEY_RULE_NAME, &self.rule_name);
            row.field(
                constants::KEY_INPUT_CARDINALITIES,
                &self.input_cardinalities,
            );
            row.field(constants::KEY_NODE_NAMESPACE, &self.node_namespace);
        });
    }
}
#[cfg(feature = "diagnostic-row-progress")]
impl Encode for RouteCounters {
    fn encode(&self, out: &mut RouteDocument) {
        object(out, |row| {
            row.field(constants::COUNTER_INPUT_ROWS, &self.input_rows);
            row.field(constants::COUNTER_EMITTED_ROWS, &self.emitted_rows);
            row.field(
                constants::COUNTER_PREDICATE_EVALUATIONS,
                &self.predicate_evaluations,
            );
            row.field(constants::COUNTER_INDEX_PROBES, &self.index_probes);
            row.field(
                constants::COUNTER_INDEX_ROWS_EXAMINED,
                &self.index_chain_rows_examined,
            );
            row.field(constants::COUNTER_EQUALITY_MATCHES, &self.equality_matches);
            row.field(
                constants::COUNTER_JOIN_CANDIDATE_PAIRS,
                &self.join_candidate_pairs,
            );
            row.field(
                constants::COUNTER_KERNEL_OUTPUT_ROWS,
                &self.kernel_output_rows,
            );
            row.field(
                constants::COUNTER_POSITION_ROWS_YIELDED,
                &self.position_rows_yielded,
            );
        });
    }
}
