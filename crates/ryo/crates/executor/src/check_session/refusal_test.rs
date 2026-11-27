//! The faults a provider is refused for: a cell outside its file, and the sentence that says so.

use super::refusal::{Extent, ProviderFault};
use ryo_relations::domain::relations::types::CellKind;

const HOME: u32 = 3;

fn extent() -> Extent<'static> {
    Extent { provider: "words", home: HOME, elements: 10, bytes: 100 }
}

fn node(file: u32, place: u32) -> u64 {
    (u64::from(file) << 32) | u64::from(place)
}

fn span(start: u32, end: u32) -> u64 {
    (u64::from(start) << 32) | u64::from(end)
}

#[test]
fn a_node_of_the_file_within_its_table_is_no_fault() {
    assert_eq!(extent().fault(CellKind::Node, "id", &[node(HOME, 0), node(HOME, 9), u64::MAX]), None);
}

#[test]
fn a_node_of_another_file_is_a_fault() {
    assert_eq!(extent().fault(CellKind::Node, "id", &[node(HOME + 1, 0)]), Some(ProviderFault::NodeOfAnotherFile));
}

#[test]
fn a_node_past_the_last_position_is_a_fault_and_a_diagnostic_identity_is_not() {
    assert_eq!(extent().fault(CellKind::Node, "id", &[node(HOME, 10)]), Some(ProviderFault::NodeOutsideTable));
    assert_eq!(extent().fault(CellKind::Node, "id", &[node(HOME, 0x8000_0000 + 4)]), None);
}

#[test]
fn a_span_must_lie_in_the_bytes_of_the_file() {
    assert_eq!(extent().fault(CellKind::Span, "span", &[span(0, 100), span(5, 5), span(u32::MAX, 0)]), None);
    assert_eq!(extent().fault(CellKind::Span, "span", &[span(0, 101)]), Some(ProviderFault::SpanOutsideFile));
    assert_eq!(extent().fault(CellKind::Span, "span", &[span(7, 3)]), Some(ProviderFault::SpanOutsideFile));
}

#[test]
fn the_file_column_names_the_file_visited() {
    assert_eq!(extent().fault(CellKind::File, "file", &[u64::from(HOME)]), None);
    assert_eq!(extent().fault(CellKind::File, "file", &[u64::from(HOME) + 1]), Some(ProviderFault::FileNotVisited));
    assert_eq!(extent().fault(CellKind::File, "target", &[u64::from(HOME) + 1]), None);
}
