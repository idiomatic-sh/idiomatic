//! The linked language hands the engine its numbered visit: the one way the engine reads the language's files.

#[test]
fn the_linked_language_hands_out_its_numbered_visit() {
    let linked = ryo_markdown::linked();
    let engine = linked.parse_engine();
    assert!(
        engine.file_relations().and_then(|relations| relations.numbered()).is_some(),
        "the language hands out no numbered visit"
    );
}
