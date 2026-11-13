//! Drive a provider's parse contract, then dispatch each requested relation through its own reader.
//! The table is traversed once; what a language's `Semantics` answers decides both absence answers and reads of its optional relations.

use super::constants::COUNT;
use super::errors::ParseError;
use super::interface::ParseUseCase;
use super::types::{
    ParseEngine, ParseRequest, Recognition, Slot, absence_in, expands, reads_of, slot_of,
    unsupported, visit,
};
use crate::domain::answers::types::{Expansions, Reads};
use crate::domain::builders::service::{discarding_builder, table_builder};
use crate::domain::completion::service::completion;
use crate::domain::elements::service::{engine_mut, expanded_positions, kind_at, positions};
use crate::domain::elements::types::{CompletionRequirement, Elements, Tree};
use crate::domain::relation_masks::service::{add, none, places};
use crate::domain::relation_masks::types::RelationSet;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::catalog::types::OptionalCapability;
use ryo_relations::domain::relations::service::{
    KIND_LIMIT, all, number_of, optional_capability, provenance_of,
};
use ryo_relations::domain::relations::types::{FileId, KindBits, Provenance, Relation, RowWriter};

/// The kinds the readers of the language look at, every reader's declared `kinds:` together. A reader of every element names none: it
/// reads all and models nothing, so it does not count.
#[must_use]
pub(crate) fn kinds_read_in<L>(language: &L) -> Vec<KindBits>
where
    L: ParseUseCase + ?Sized,
{
    let mut found = Vec::new();
    for relation in all() {
        if let Some(Reads::Kinds(kinds)) = slot_of(language, *relation).map(|slot| reads_of(&slot))
        {
            found.extend_from_slice(kinds);
        }
    }
    found
}

/// Every optional promise of the language: each relation of its files' trees it answers through its `Semantics` other than `NotYetRead`,
/// in the catalog's order, then the project's it lists in `capabilities`. The first are read from the answers, never from a list, so a
/// language's promise and its answer cannot disagree.
#[must_use]
pub(crate) fn promises_in<L>(language: &L) -> Vec<OptionalCapability>
where
    L: ParseUseCase + ?Sized,
{
    let mut promised: Vec<OptionalCapability> = all()
        .iter()
        .filter(|relation| provenance_of(**relation) == Provenance::Trees)
        .filter(|relation| absence_in(language, **relation) != Some(AbsenceCause::NotYetRead))
        .filter_map(|relation| optional_capability(*relation))
        .collect();
    promised.extend_from_slice(language.capabilities());
    promised
}

/// The language's requirement when a wanted reader looks at what completion adds; else none: no model is prepared for it.
fn demanded<L: ParseUseCase + ?Sized>(language: &L, wanted: &[Relation]) -> CompletionRequirement {
    let mut slots = wanted
        .iter()
        .filter_map(|relation| slot_of(language, *relation));
    let reads_added = slots.any(|slot| expands(&slot) == Expansions::Included);
    reads_added
        .then(|| language.completion_requirement())
        .unwrap_or_default()
}

fn build_recognized<L: ParseUseCase + ?Sized>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    requirement: CompletionRequirement,
) -> Result<(), ParseError> {
    let mut builder = table_builder(table, symbols, request.bytes);
    language.read_tree(&request, &mut builder)?;
    crate::application::completion::service::complete::<L::FileModel>(
        completion(table, symbols, request.bytes, request.file),
        ByteSlice(language.configuration_identity().0.as_bytes()),
        requirement,
        |tree, model| language.file_model(tree, model),
        |completed| language.complete(&request, completed),
        |tree, from, model| language.extend_file_model(tree, from, model),
    );
    Ok(())
}

/// Does only what the language's library does for the file the request names, through a builder that builds nothing: no table, no
/// name, no row. The allocation row is that reading a file allocates no more than this, so the bound is the language's
/// `read_tree` itself, never a body written beside it.
///
/// # Errors
///
/// What the language's library says of a file it refuses: a baseline taken of a refused file measures no read.
pub(crate) fn baseline_in<L>(language: &L, request: &ParseRequest<'_>) -> Result<(), ParseError>
where
    L: ParseUseCase + ?Sized,
{
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = discarding_builder(&mut elements, &mut symbols, request.bytes);
    language.read_tree(request, &mut builder)
}

/// Builds the tree of one file in `table`, the worker's own, completed for what the readers of `wanted` look at: the language reads
/// the file with its library into the builder the engine makes. No row is written: the engine reads the tree with the readers the
/// language answers.
///
/// # Errors
///
/// `Unsupported` when the language does not read this file, and what its library refuses.
pub(crate) fn build_wanted_in<L>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
) -> Result<(), ParseError>
where
    L: ParseUseCase + ?Sized,
{
    if language.recognizes(&request) == Recognition::Declines {
        return Err(ParseError::Unsupported);
    }
    let requirement = demanded(language, wanted);
    build_recognized(language, request, table, symbols, requirement)
}

/// Builds the tree of a file explicitly assigned to the language, completed for what the readers of `wanted` look at: no row is
/// written.
///
/// # Errors
///
/// `Unsupported` unless the language reads the file when selected, and what its `build` answers.
pub(crate) fn build_explicit_in<L: ParseUseCase + ?Sized>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
) -> Result<(), ParseError> {
    if language.recognizes_explicit(&request) == Recognition::Declines {
        return Err(ParseError::Unsupported);
    }
    build_recognized(
        language,
        request,
        table,
        symbols,
        demanded(language, wanted),
    )
}

/// Reads one file: the language builds its tree, and the rows of the wanted relations are written into `out`.
///
/// # Errors
///
/// What the language's `build` answers.
pub(crate) fn read_in<L>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) -> Result<(), ParseError>
where
    L: ParseUseCase + ?Sized,
{
    build_wanted_in(language, request, table, symbols, wanted)?;
    walk_in(
        language,
        table,
        symbols,
        request.bytes,
        request.file,
        wanted,
        out,
    );
    Ok(())
}

/// Drive the same parser and relation readers after validated explicit provider selection.
pub(crate) fn read_explicit_in<L: ParseUseCase + ?Sized>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) -> Result<(), ParseError> {
    build_explicit_in(language, request, table, symbols, wanted)?;
    walk_in(
        language,
        table,
        symbols,
        request.bytes,
        request.file,
        wanted,
        out,
    );
    Ok(())
}

/// Writes the rows of the wanted relations for the file whose table is `elements`: every element
/// is given, in order, to the readers that look at its kind. The table is only read. The model of the language is the worker's own: it
/// is made once, kept with the table and given back to the language to refill for each file.
pub(crate) fn walk_in<L>(
    language: &L,
    elements: &mut Elements,
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    file: FileId,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) where
    L: ParseUseCase + ?Sized,
{
    let mut model: Box<L::FileModel> =
        crate::application::completion::service::take_model(elements);
    if wanted.contains(&Relation::SourceLocation) {
        crate::domain::location::service::index(
            &mut engine_mut(elements).line_starts,
            crate::domain::location::types::SourceLineBytes(source.0),
        );
    }
    walk_with(
        language, elements, &mut model, symbols, source, file, wanted, out,
    );
    crate::application::completion::service::keep_model(elements, model);
}

/// Writes the rows of the wanted relations for the file whose table is `elements`, given the model the worker keeps for the language.
#[expect(
    clippy::too_many_arguments,
    reason = "the arguments of `walk_in` and the model the worker keeps for the language"
)]
fn walk_with<L>(
    language: &L,
    elements: &mut Elements,
    model: &mut L::FileModel,
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    file: FileId,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) where
    L: ParseUseCase + ?Sized,
{
    let mut slots: [Option<Slot<L::FileModel>>; COUNT] = std::array::from_fn(|_| None);
    let mut masks: [RelationSet; KIND_LIMIT] = [none(); KIND_LIMIT];
    let mut reads_any = false;
    for relation in wanted {
        if optional_capability(*relation).is_some()
            && absence_in(language, *relation) == Some(AbsenceCause::NotYetRead)
        {
            unsupported(*relation, file, out);
            continue;
        }
        let place = number_of(*relation);
        let Some(each) = slot_of(language, *relation) else {
            continue;
        };
        match reads_of(&each) {
            Reads::Every => masks.iter_mut().for_each(|mask| add(mask, place)),
            Reads::Kinds(kinds) => {
                for kind in kinds {
                    if let Some(mask) = masks.get_mut(usize::from(kind.0)) {
                        add(mask, place);
                    }
                }
            }
        }
        if let Some(slot) = slots.get_mut(place.0) {
            *slot = Some(each);
        }
        reads_any = true;
    }
    if !reads_any {
        return;
    }
    if language.completion_requirement()
        == crate::domain::elements::types::CompletionRequirement::PreparedModel
    {
        crate::application::completion::service::prepare(
            elements,
            source,
            file,
            ByteSlice(language.configuration_identity().0.as_bytes()),
            model,
            |tree, model| language.file_model(tree, model),
        );
    } else {
        crate::application::completion::service::initialize(
            elements,
            source,
            file,
            model,
            |tree, model| language.file_model(tree, model),
        );
    }
    let tree = Tree {
        elements,
        source,
        file,
    };
    // Preserve native-before-expanded ordering through one dispatch loop. No added table
    // is traversed unless at least one requested reader opted into added occurrences.
    let expanded = slots
        .iter()
        .flatten()
        .any(|each| expands(each) == Expansions::Included);
    let occurrences = positions(elements).map(|position| (position, false)).chain(
        expanded_positions(elements)
            .take(if expanded { usize::MAX } else { 0 })
            .map(|position| (position, true)),
    );
    for (position, added) in occurrences {
        let Some(mask) = masks.get(usize::from(kind_at(elements, position).0)) else {
            continue;
        };
        for place in places(mask) {
            if let Some(Some(each)) = slots.get(place.0)
                && (!added || expands(each) == Expansions::Included)
            {
                visit(each, &tree, model, symbols, position, out);
            }
        }
    }
}

/// What the language answers for `relation`, asked of the engine's interface: why it does not read it from a tree, or nothing.
#[must_use]
pub fn absence(language: &dyn ParseEngine, relation: Relation) -> Option<AbsenceCause> {
    language.absence_of(relation)
}

/// The kinds the readers of the language look at, asked of the engine's interface.
#[must_use]
pub fn kinds_read(language: &dyn ParseEngine) -> Vec<KindBits> {
    language.reader_kinds()
}

/// Builds the whole tree of the file in `table`, completed for every reader, asked of the engine's interface.
///
/// # Errors
///
/// What the language's `build` answers.
pub fn build(
    language: &dyn ParseEngine,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
) -> Result<(), ParseError> {
    language.build_file(request, table, symbols, all())
}

/// Reads one file and writes the rows of the wanted relations into `out`, asked of the engine's interface.
///
/// # Errors
///
/// What the language's `build` answers.
pub fn read(
    language: &dyn ParseEngine,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) -> Result<(), ParseError> {
    language.read_file(request, table, symbols, wanted, out)
}

/// Writes the rows of the wanted relations for the file whose table is `elements`, asked of the engine's interface.
pub fn walk(
    language: &dyn ParseEngine,
    elements: &mut Elements,
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    file: FileId,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) {
    language.walk_file(elements, symbols, source, file, wanted, out);
}

/// Does only what the language's library does for the file, asked of the engine's interface.
///
/// # Errors
///
/// What the language's library says of a file it refuses.
pub fn baseline(language: &dyn ParseEngine, request: &ParseRequest<'_>) -> Result<(), ParseError> {
    language.baseline_file(request)
}
