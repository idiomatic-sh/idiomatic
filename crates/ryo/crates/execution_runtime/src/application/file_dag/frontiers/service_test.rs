use super::service::exact_required_files;
use super::types::FileAccounting;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn a_complete_bucket_requires_every_declared_project_file() {
    let required = [FileId(0), FileId(1)];
    assert_eq!(
        exact_required_files(&required, &[FileId(0)].into()),
        FileAccounting::Inexact
    );
    assert_eq!(
        exact_required_files(&required, &[FileId(0), FileId(1)].into()),
        FileAccounting::Exact
    );
    assert_eq!(
        exact_required_files(&required, &[FileId(0), FileId(1), FileId(2)].into()),
        FileAccounting::Inexact
    );
}

/// The stored words of a one-column table of numbers, in an allocation of their own.
fn number_words(
    numbers: &[i64],
) -> std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords> {
    let table = ryo_values::domain::rows::types::Table {
        kinds: vec![ryo_relations::domain::relations::types::CellKind::Number],
        cells: numbers
            .iter()
            .map(|number| {
                ryo_relations::domain::relations::types::Value::Number(
                    ryo_relations::domain::relations::types::Integer(*number),
                )
            })
            .collect(),
        count: ryo_values::domain::rows::types::RowCount(numbers.len()),
    };
    let mut words = ryo_relations::domain::relations::types::Words::default();
    ryo_values::domain::table_words::service::encode_table(&table, &[], &mut words);
    std::sync::Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
        ryo_values::domain::table_words::service::pack(
            ryo_values::domain::table_words::types::WordSlice(&words.0),
        ),
    ))
}

fn packed(
    input: ryo_values::domain::plan_parts::types::Input,
    words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
) -> super::types::PackedFrontier {
    super::types::PackedFrontier {
        input,
        words,
        mode: ryo_values::domain::table_words::types::DecodeMode::Permissive,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState::Complete,
    }
}

/// The input an admitted frontier answers.
fn input_of(
    admitted: &Result<
        super::types::FrontierInput,
        ryo_execution_domain::domain::frontiers::types::RefusedFrontier,
    >,
) -> Option<ryo_values::domain::plan_parts::types::Input> {
    match admitted {
        Ok(super::types::FrontierInput::Decoded { frontier, .. }) => Some(frontier.input),
        Ok(super::types::FrontierInput::Compact { input, .. }) => Some(*input),
        Ok(super::types::FrontierInput::Packed(packed)) => Some(packed.input),
        Err(refused) => Some(refused.input),
    }
}

#[test]
fn content_equal_words_are_decoded_once_and_relabelled_for_each_root() {
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::Input;
    let decodes = super::types::FrontierDecodes::new(std::sync::Arc::new(
        crate::outbound::digest::fixtures::fake::DigestFake,
    ));
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let first = decodes.admit(
        packed(Input::Relation(Relation::Function), number_words(&[1, 2])),
        &mut symbols,
        super::service::admit,
    );
    let second = decodes.admit(
        packed(Input::Relation(Relation::Identifier), number_words(&[1, 2])),
        &mut symbols,
        super::service::admit,
    );
    assert_eq!(
        decodes.count(),
        super::types::DecodeCount(1),
        "two roots reading equal words in their own buckets decode once"
    );
    assert_eq!(input_of(&first), Some(Input::Relation(Relation::Function)));
    assert_eq!(
        input_of(&second),
        Some(Input::Relation(Relation::Identifier)),
        "the shared rows answer the second root's own input"
    );
    match (first, second) {
        (
            Ok(super::types::FrontierInput::Decoded { frontier: one, .. }),
            Ok(super::types::FrontierInput::Decoded { frontier: two, .. }),
        ) => {
            assert!(
                std::sync::Arc::ptr_eq(&one.table, &two.table),
                "the second root receives the first root's rows, not a copy"
            );
        }
        _ => panic!("permissive relation frontiers decode to rows"),
    }
}

#[test]
fn other_words_another_shape_or_another_symbol_generation_decode_again() {
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::{Input, StepId};
    let decodes = super::types::FrontierDecodes::new(std::sync::Arc::new(
        crate::outbound::digest::fixtures::fake::DigestFake,
    ));
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let _ = decodes.admit(
        packed(Input::Relation(Relation::Function), number_words(&[1, 2])),
        &mut symbols,
        super::service::admit,
    );
    let _ = decodes.admit(
        packed(Input::Relation(Relation::Function), number_words(&[3])),
        &mut symbols,
        super::service::admit,
    );
    assert_eq!(
        decodes.count(),
        super::types::DecodeCount(2),
        "different words are another population"
    );
    let _ = decodes.admit(
        packed(Input::Step(StepId(0)), number_words(&[1, 2])),
        &mut symbols,
        super::service::admit,
    );
    assert_eq!(
        decodes.count(),
        super::types::DecodeCount(3),
        "a step decodes the same words to compact rows of its own"
    );
    let mut other = ryo_language::domain::symbols::types::Symbols::default();
    let _ = decodes.admit(
        packed(Input::Relation(Relation::Function), number_words(&[1, 2])),
        &mut other,
        super::service::admit,
    );
    assert_eq!(
        decodes.count(),
        super::types::DecodeCount(4),
        "names decoded in one symbol generation are never handed to another"
    );
}

#[test]
fn a_decode_releases_its_packed_words_and_an_equal_second_root_decodes_nothing() {
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::Input;
    let decodes = super::types::FrontierDecodes::new(std::sync::Arc::new(
        crate::outbound::digest::fixtures::fake::DigestFake,
    ));
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let words = number_words(&[1, 2]);
    let held = std::sync::Arc::downgrade(&words);
    let first = decodes.admit(
        packed(Input::Relation(Relation::Function), words),
        &mut symbols,
        super::service::admit,
    );
    assert!(
        held.upgrade().is_none(),
        "once decoded, neither the scope nor the rows hold the packed words"
    );
    let second = decodes.admit(
        packed(Input::Relation(Relation::Identifier), number_words(&[1, 2])),
        &mut symbols,
        super::service::admit,
    );
    assert_eq!(
        decodes.count(),
        super::types::DecodeCount(1),
        "a second root with equal words decodes nothing"
    );
    assert_eq!(input_of(&first), Some(Input::Relation(Relation::Function)));
    assert_eq!(
        input_of(&second),
        Some(Input::Relation(Relation::Identifier))
    );
}
mod cut_refusal;
