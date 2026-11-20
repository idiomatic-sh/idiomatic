//! The `walk` contract's cases against today's reading: over the corpus the language's fixtures pin (real modules of the standard
//! library and of two libraries), every relation the walk fills has the rows today's reading writes, and answers in part exactly
//! where today's does. Today's table also embeds the expressions of strings it may read as annotations, after the file's own
//! tree; the walk embeds none yet, so today's rows on those elements are left out and counted.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::today::{Read, read, read_keys, read_rows, rows_by_relation};
use crate::fixtures::walking::{Seen, walked, walked_keys, walked_rows};
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{self, FactAnswer, NodeTable, Relation, WalkError};

/// The relations the walk fills with an item of its own.
const FILLED: &[Relation] = &[
    Relation::BindingEvent,
    Relation::BindingRead,
    Relation::BindingShadow,
    Relation::BindingUse,
    Relation::BraceFormatField,
    Relation::BraceFormatParse,
    Relation::BytesLiteral,
    Relation::CallArgument,
    Relation::Comment,
    Relation::CompilerFeature,
    Relation::ConstantKey,
    Relation::Decorates,
    Relation::Defines,
    Relation::ExpressionShape,
    Relation::ExpressionType,
    Relation::FormatField,
    Relation::FormatParse,
    Relation::FormatSpecification,
    Relation::Function,
    Relation::FunctionTerminal,
    Relation::Identifier,
    Relation::ImplicitSymbol,
    Relation::IndentRegion,
    Relation::Inherits,
    Relation::InterpolatedStringLiteral,
    Relation::LanguageTarget,
    Relation::LiteralExpressionParse,
    Relation::LiteralProperties,
    Relation::NumericLiteral,
    Relation::Parameter,
    Relation::QualifiedName,
    Relation::RawFormatField,
    Relation::RawFormatSegment,
    Relation::Reference,
    Relation::Resolves,
    Relation::Scope,
    Relation::SourceLocation,
    Relation::StandardLibraryImport,
    Relation::StaticImport,
    Relation::StringLiteral,
    Relation::SyntaxError,
    Relation::TokenLink,
];

/// The folder of the fixtures whose sources are compared.
type Input = &'static str;
/// What differs, one line for each relation and file that differs; empty when every relation's rows are today's.
type Value = String;
type Failure = WalkError;

/// The rows of the cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_filled_relation_has_today_s_rows_over_the_pinned_corpus"),
    word: Word::Succeeds,
    input: || "corpus/",
    expect: Expect::Value(String::new()),
}];

/// Whether a row names an element today's table holds outside the file's own tree: an embedded expression's.
fn embedded(row: &[Seen]) -> bool {
    row.iter()
        .any(|cell| matches!(cell, Seen::Other(id) if *id < u64::from(u32::MAX / 2)))
}

/// What differs between today's rows and the walk's for one source.
fn differences(path: &str, bytes: &[u8]) -> Result<String, WalkError> {
    let linked = crate::linked();
    let mut today = Read::default();
    read(
        linked.parse_engine().as_ref(),
        "input.py",
        bytes,
        FILLED,
        &mut today,
    )
    .map_err(|_refused| WalkError::Library)?;
    let today_keys = read_keys(&today.elements);
    let today_rows = rows_by_relation(&today);
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut found = String::new();
    walked(
        &crate::walk(),
        "input.py",
        bytes,
        FILLED,
        &mut table,
        &mut scratch,
        &mut |mut file| {
            let mut words = languages::facts::file_words(&languages::facts::base_words(&[]));
            let keys = walked_keys(file.table);
            for relation in FILLED {
                let (before, left_out): (Vec<Vec<Seen>>, Vec<Vec<Seen>>) =
                    read_rows(&today, &today_rows, bytes, &today_keys, *relation)
                        .into_iter()
                        .partition(|row| !embedded(row));
                let (after, answer) = walked_rows(&mut file, &keys, *relation, &mut words);
                // A row today's reading names by an embedded element (a class named by its last member) is left out on both
                // sides: the walk names the same element's class by a member of the file's own tree.
                let after: Vec<Vec<Seen>> = after
                    .into_iter()
                    .filter(|row| !left_out.iter().any(|out| out.first() == row.first()))
                    .collect();
                let partial_today = today
                    .rows
                    .partials
                    .iter()
                    .any(|(each, _, _)| each == relation);
                let answered_alike = match answer {
                    FactAnswer::Answered => !partial_today,
                    FactAnswer::Partial(_) => partial_today,
                    FactAnswer::Refused(_) => false,
                };
                if before != after || !answered_alike {
                    found.push_str(&format!(
                    "{path}: {relation:?}: {} rows today, {} walked, answered alike: {answered_alike}\n",
                    before.len(),
                    after.len()
                ));
                    for (side, have, other) in
                        [("today", &before, &after), ("walked", &after, &before)]
                    {
                        for row in have.iter().filter(|row| !other.contains(row)).take(3) {
                            found.push_str(&format!("  only {side}: {row:?}\n"));
                        }
                    }
                }
            }
        },
    )?;
    Ok(found)
}

fn subject(folder: Input) -> Result<Value, Failure> {
    fixture_sources()
        .map_err(|_unread| WalkError::Library)?
        .iter()
        .filter(|(path, _)| path.starts_with(folder))
        .map(|(path, bytes)| differences(path, bytes))
        .collect()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
