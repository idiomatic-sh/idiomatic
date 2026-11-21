//! The `walk` contract's cases against today's reading: over the corpus the language's fixtures pin (real modules of the standard
//! library and of two libraries), every relation the walk fills has the rows today's reading writes, and answers in part exactly
//! where today's does. Today's table embeds the expression of every string it may read as a type expression, after the file's
//! own tree, and its readers leave out the ones it does not; the walk embeds only the ones the language reads as one.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::today::{
    Read, read, read_keys_all, read_own_count, read_rows, rows_by_relation,
};
use crate::fixtures::walking::{ElementKey, Seen, walk_words, walked, walked_keys, walked_rows};
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{FactAnswer, NodeTable, Relation, WalkError};

/// The relations the walk fills with an item of its own.
const FILLED: &[Relation] = &[
    Relation::BindingEvent,
    Relation::BindingRead,
    Relation::BindingResolution,
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
    Relation::EmbeddedExpression,
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
    Relation::LookupScope,
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

/// The relations whose rows on the nodes of embedded expressions the walk does not write as today's reading does: today's
/// rows on those nodes are left out for them and the walk's are held to the file's own tree. Every other relation is compared
/// on embedded nodes too. `source-location` is here because today's table embeds every string it may read as a type
/// expression and the walk the ones the language reads as one; the two expression relations are not written on embedded
/// nodes yet, nor are `constant-key` and `literal-properties` (SPIKE ryo-fmx3.1, D: the walk's open rows on embedded nodes).
const NOT_EMBEDDED: &[Relation] = &[
    Relation::ConstantKey,
    Relation::ExpressionShape,
    Relation::ExpressionType,
    Relation::LiteralProperties,
    Relation::SourceLocation,
];

/// Whether a row names an element today's table holds outside the file's own tree: an embedded expression's.
fn embedded(row: &[Seen], outside: &[ElementKey]) -> bool {
    row.iter()
        .any(|cell| matches!(cell, Seen::Element(key) if outside.contains(key)))
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
    let today_keys = read_keys_all(&today.elements);
    let outside: Vec<ElementKey> = today_keys
        .0
        .get(read_own_count(&today.elements)..)
        .unwrap_or_default()
        .to_vec();
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
            let mut words =
                walk_words(&crate::walk()).expect("each fixed name of the walk is stated once");
            let keys = walked_keys(file.table);
            for relation in FILLED {
                let (before, left_out): (Vec<Vec<Seen>>, Vec<Vec<Seen>>) =
                    read_rows(&today, &today_rows, bytes, &today_keys, *relation)
                        .into_iter()
                        .partition(|row| !NOT_EMBEDDED.contains(relation) || !embedded(row, &outside));
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
                    FactAnswer::Answered(_) => !partial_today,
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
