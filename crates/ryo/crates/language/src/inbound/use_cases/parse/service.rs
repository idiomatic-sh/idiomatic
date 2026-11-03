//! Drive a provider's parse contract, then dispatch each requested relation through its own reader.
//! The table is traversed once; optional capability promises gate both absence answers and reads.

use super::constants::{COUNT, WORDS};
use super::errors::ParseError;
use super::interface::ParseUseCase;
use super::types::{ParseEngine, ParseRequest, Slot};
use crate::domain::answers::types::{AbsenceCause, Answer, Expansions, Reads};
use crate::domain::completion::service::completion;
use crate::domain::elements::service::{discarding_builder, engine_mut, kind_at};
use crate::domain::elements::service::{expanded_positions, positions, table_builder};
use crate::domain::elements::types::{Elements, Tree};
use crate::domain::relations::service::{KIND_LIMIT, all, number_of, optional_capability};
use crate::domain::relations::types::{Count, FileId, KindBits, Relation, RowWriter};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::types::Recognition;

/// A set of relations, a bit for each, by the relation's number.
type Mask = [u64; WORDS];

/// What the reading of a file is made of: a slot for each syntax relation holding its reader, and the
/// functions over a slot. Generated from the catalog's one list.
macro_rules! syntax_slots {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest_variant:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        /// What the language answers for `relation`: its reader when it reads it from a tree.
        fn slot_of<L>(language: &L, relation: Relation) -> Option<Slot<L::FileModel>> where L: ParseUseCase + ?Sized {
            if optional_capability(relation)
                .is_some_and(|capability| !language.capabilities().contains(&capability)) {
                return None;
            }
            match relation {
                $( Relation::$variant => match language.$module().as_ref() {
                    Answer::Reads(reader) => Some(Slot::$variant(*reader)),
                    Answer::NotInThisLanguage(_) => None,
                }, )*
                $( Relation::$manifest_variant => None, )*
                $( Relation::$engine_variant => None, )*
            }
        }

        fn unsupported(relation: Relation, file: FileId, out: &mut dyn RowWriter) {
            match relation {
                $( Relation::$variant => <dyn RowWriter as crate::domain::relations::types::Rows<$crate::domain::relations::collections::relations::$module::item::$row>>::partial_at(
                    out, file, crate::domain::relations::types::ReportText(super::constants::UNAVAILABLE_CAPABILITY), crate::domain::relations::types::Missing::Any), )*
                $( Relation::$manifest_variant => {}, )*
                $( Relation::$engine_variant => {}, )*
            }
        }

        /// Which elements the reader in `slot` looks at.
        fn reads_of<M>(slot: &Slot<M>) -> Reads {
            match slot {
                $( Slot::$variant(reader) => reader.reads, )*
            }
        }

        /// Gives the element at `position` to the reader in `slot`, which writes its relation's rows into `out`.
        fn visit<M>(
            slot: &Slot<M>,
            tree: &Tree<'_>,
            model: &M,
            symbols: &mut Symbols,
            position: crate::domain::relations::types::Position,
            out: &mut dyn RowWriter,
        ) where M: 'static {
            match slot {
                $( Slot::$variant(reader) => reader.visit.visit(tree, model, symbols, position, out), )*
            }
        }

        /// Whether the reader in `slot` also looks at the elements the language added for reading.
        fn expands<M>(slot: &Slot<M>) -> Expansions where M: 'static {
            match slot {
                $( Slot::$variant(reader) => reader.expansions, )*
            }
        }

        /// Why the language does not read `relation` from a tree, or nothing when it does or when the
        /// relation is not one of the trees'.
        #[must_use]
        pub fn absence_in<L>(language: &L, relation: Relation) -> Option<AbsenceCause> where L: ParseUseCase + ?Sized {
            if optional_capability(relation)
                .is_some_and(|capability| !language.capabilities().contains(&capability)) {
                return Some(AbsenceCause::NotYetRead);
            }
            match relation {
                $( Relation::$variant => match language.$module().as_ref() {
                    Answer::Reads(_) => None,
                    Answer::NotInThisLanguage(why) => Some(*why),
                }, )*
                $( Relation::$manifest_variant => None, )*
                $( Relation::$engine_variant => None, )*
            }
        }
    };
}
crate::each_relation!(syntax_slots);

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

/// Sets the bit of the relation numbered `place`.
fn set(mask: &mut Mask, place: Count) {
    if let Some(word) = mask.get_mut(place.0 / 64) {
        *word |= 1 << (place.0 % 64);
    }
}

/// Builds the tree of the file in `table`, the worker's own: the language reads the file with its library into the builder the
/// engine makes, and completes the table. The rows are not written here: the engine reads the tree with the readers the language
/// answers.
///
/// # Errors
///
/// `Unsupported` when the language does not read this file, and what its library refuses.
pub(crate) fn build_in<L>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
) -> Result<(), ParseError>
where
    L: ParseUseCase + ?Sized,
{
    if language.recognizes(&request) == Recognition::Declines {
        return Err(ParseError::Unsupported);
    }
    build_recognized(language, request, table, symbols)
}

fn build_recognized<L: ParseUseCase + ?Sized>(
    language: &L,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
) -> Result<(), ParseError> {
    let mut builder = table_builder(table, symbols, request.bytes);
    language.read_tree(&request, &mut builder)?;
    crate::application::completion::service::complete::<L::FileModel>(
        completion(table, symbols, request.bytes, request.file),
        language.configuration_identity().0,
        language.completion_requirement(),
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
    build_in(language, request, table, symbols)?;
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
    if language.recognizes_explicit(&request) == Recognition::Declines {
        return Err(ParseError::Unsupported);
    }
    build_recognized(language, request, table, symbols)?;
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
    let mut masks: [Mask; KIND_LIMIT] = [[0; WORDS]; KIND_LIMIT];
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
            Reads::Every => masks.iter_mut().for_each(|mask| set(mask, place)),
            Reads::Kinds(kinds) => {
                for kind in kinds {
                    if let Some(mask) = masks.get_mut(usize::from(kind.0)) {
                        set(mask, place);
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
            language.configuration_identity().0,
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
        for (word, bits) in mask.iter().enumerate() {
            let mut bits = *bits;
            while bits != 0 {
                let place =
                    word * 64 + usize::try_from(bits.trailing_zeros()).unwrap_or(usize::MAX);
                bits &= bits - 1;
                if let Some(Some(each)) = slots.get(place)
                    && (!added || expands(each) == Expansions::Included)
                {
                    visit(each, &tree, model, symbols, position, out);
                }
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

/// Builds the tree of the file in `table`, asked of the engine's interface.
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
    language.build_file(request, table, symbols)
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
