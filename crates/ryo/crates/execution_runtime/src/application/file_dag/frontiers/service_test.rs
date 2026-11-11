//! Packed frontiers decode once per equal words, shape and symbol generation, and each root reads its own input.
use crate::application::file_dag::frontiers::decodes::service::FrontierDecodes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input as PlanInput, StepId};
use std::borrow::Cow;
use std::sync::Arc;

mod cut_refusal;
mod required_files;

/// The stored words of a one-column table of numbers, in an allocation of their own.
fn number_words(numbers: &[i64]) -> Arc<ryo_values::domain::stored_facts::types::KeptWords> {
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
    Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
        ryo_values::domain::table_words::service::pack(
            ryo_values::domain::table_words::types::WordSlice(&words.0),
        ),
    ))
}

fn packed(
    input: PlanInput,
    words: Arc<ryo_values::domain::stored_facts::types::KeptWords>,
) -> super::types::PackedFrontier {
    super::types::PackedFrontier {
        input,
        words,
        mode: ryo_values::domain::table_words::types::DecodeMode::Permissive,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState::Complete,
    }
}

/// One packed frontier handed to the decodes: the input it is packed for, its numbers, and the
/// symbol generation (an index into the run's symbol tables) it is admitted in.
#[derive(Clone, Copy, Debug)]
struct Admission {
    input: PlanInput,
    numbers: &'static [i64],
    generation: usize,
}

/// What the decodes answered over the admissions.
#[derive(Debug, PartialEq)]
struct Decoded {
    /// The decode count after each admission.
    counts: Cow<'static, [usize]>,
    /// The input each admission answered, `None` when it answered none.
    inputs: Cow<'static, [Option<PlanInput>]>,
    /// Whether the first two admissions decoded to the very same rows.
    shared_rows: bool,
    /// Whether the first admission's packed words were released once it was admitted.
    first_words_released: bool,
}

const fn admission(input: PlanInput, numbers: &'static [i64], generation: usize) -> Admission {
    Admission {
        input,
        numbers,
        generation,
    }
}

const FUNCTION: PlanInput = PlanInput::Relation(Relation::Function);
const IDENTIFIER: PlanInput = PlanInput::Relation(Relation::Identifier);
const STEP: PlanInput = PlanInput::Step(StepId(0));

type Input = &'static [Admission];
type Value = Decoded;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("content_equal_words_are_decoded_once_and_relabelled_for_each_root"),
        word: Word::Succeeds,
        input: || {
            const {
                &[
                    admission(FUNCTION, &[1, 2], 0),
                    admission(IDENTIFIER, &[1, 2], 0),
                ]
            }
        },
        expect: Expect::Value(Decoded {
            counts: Cow::Borrowed(&[1, 1]),
            inputs: Cow::Borrowed(&[Some(FUNCTION), Some(IDENTIFIER)]),
            shared_rows: true,
            first_words_released: true,
        }),
    },
    Case {
        name: CaseName("other_words_another_shape_or_another_symbol_generation_decode_again"),
        word: Word::Succeeds,
        input: || {
            const {
                &[
                    admission(FUNCTION, &[1, 2], 0),
                    admission(FUNCTION, &[3], 0),
                    admission(STEP, &[1, 2], 0),
                    admission(FUNCTION, &[1, 2], 1),
                ]
            }
        },
        expect: Expect::Value(Decoded {
            counts: Cow::Borrowed(&[1, 2, 3, 4]),
            inputs: Cow::Borrowed(&[Some(FUNCTION), Some(FUNCTION), Some(STEP), Some(FUNCTION)]),
            shared_rows: false,
            first_words_released: true,
        }),
    },
    Case {
        name: CaseName(
            "a_decode_releases_its_packed_words_and_an_equal_second_root_decodes_nothing",
        ),
        word: Word::Succeeds,
        input: || {
            const {
                &[
                    admission(IDENTIFIER, &[1, 2], 0),
                    admission(FUNCTION, &[1, 2], 0),
                ]
            }
        },
        expect: Expect::Value(Decoded {
            counts: Cow::Borrowed(&[1, 1]),
            inputs: Cow::Borrowed(&[Some(IDENTIFIER), Some(FUNCTION)]),
            shared_rows: true,
            first_words_released: true,
        }),
    },
];

/// Admits each packed frontier in turn and reports the decodes' answers.
fn subject(admissions: Input) -> Result<Value, Failure> {
    let decodes = FrontierDecodes::new(Arc::new(
        crate::outbound::digest::fixtures::fake::DigestFake,
    ));
    let generations = admissions
        .iter()
        .map(|each| each.generation)
        .max()
        .map_or(0, |last| last + 1);
    let mut symbols: Vec<Symbols> = (0..generations).map(|_| Symbols::default()).collect();
    let mut counts = Vec::new();
    let mut inputs = Vec::new();
    let mut tables = Vec::new();
    let mut first_words = None;
    for each in admissions {
        let words = number_words(each.numbers);
        if first_words.is_none() {
            first_words = Some(Arc::downgrade(&words));
        }
        let admitted = decodes.admit(
            packed(each.input, words),
            &mut symbols[each.generation],
            super::service::admit,
        );
        counts.push(decodes.count().0);
        inputs.push(input_of(&admitted));
        tables.push(match admitted {
            Ok(super::types::FrontierInput::Decoded { frontier, .. }) => Some(frontier.table),
            _ => None,
        });
    }
    let shared_rows =
        matches!(tables.as_slice(), [Some(one), Some(two), ..] if Arc::ptr_eq(one, two));
    Ok(Decoded {
        counts: counts.into(),
        inputs: inputs.into(),
        shared_rows,
        first_words_released: first_words.is_some_and(|held| held.upgrade().is_none()),
    })
}

/// The input an admitted frontier answers.
fn input_of(
    admitted: &Result<
        super::types::FrontierInput,
        ryo_execution_domain::domain::frontiers::types::RefusedFrontier,
    >,
) -> Option<PlanInput> {
    match admitted {
        Ok(super::types::FrontierInput::Decoded { frontier, .. }) => Some(frontier.input),
        Ok(super::types::FrontierInput::Compact { input, .. }) => Some(*input),
        Ok(super::types::FrontierInput::Packed(packed)) => Some(packed.input),
        Err(refused) => Some(refused.input),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
