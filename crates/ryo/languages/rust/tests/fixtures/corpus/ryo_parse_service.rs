//! The `parse` service: free functions that drive an implementation of the contract. They read a file's
//! rows with the readers the implementation answers, and say what it answers for a relation. The
//! implementation builds a tree; the engine reads it, once, for every relation wanted, and gives each
//! reader the rows of its own relation to write and no other.

use super::constants::{COUNT, WORDS};
use super::errors::ParseError;
use super::interface::ParseUseCase;
use super::types::ParseRequest;
use crate::domain::answers::types::{Answer, Expansions, Index, Reader, Reads, Reason, Support};
use crate::domain::completion::service::completion;
use crate::domain::elements::service::discarding_builder;
use crate::domain::elements::service::{expanded_positions, positions, table_builder};
use crate::domain::elements::types::{Elements, Tree};
use crate::domain::relations::service::{KIND_LIMIT, all, number_of};
use crate::domain::relations::types::{Count, FileId, KindBits, Relation, RowWriter};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
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
        /// The reader of one syntax relation, whichever it is.
        #[derive(Clone, Copy)]
        enum Slot {
            $( $variant(Reader<$crate::domain::relations::collections::relations::$module::item::$variant>), )*
        }

        /// What the language answers for `relation`: its reader when it reads it from a tree.
        fn slot_of(language: &dyn ParseUseCase, relation: Relation) -> Option<Slot> {
            match relation {
                $( Relation::$variant => match language.$module() {
                    Support(Answer::Reads(reader)) => Some(Slot::$variant(reader)),
                    Support(Answer::NotInThisLanguage(_)) => None,
                }, )*
                $( Relation::$manifest_variant => None, )*
                $( Relation::$engine_variant => None, )*
            }
        }

        /// Which elements the reader in `slot` looks at.
        fn reads_of(slot: &Slot) -> Reads {
            match slot {
                $( Slot::$variant(reader) => reader.reads, )*
            }
        }

        /// What the reader in `slot` works out of the file before it is given the file's elements.
        fn indexed(slot: &Slot, tree: &Tree<'_>) -> Index {
            match slot {
                $( Slot::$variant(reader) => (reader.index)(tree), )*
            }
        }

        /// Gives the element at `position` to the reader in `slot`, which writes its relation's rows into `out`.
        fn visit(
            slot: &Slot,
            tree: &Tree<'_>,
            index: &(dyn std::any::Any + Send),
            symbols: &mut Symbols,
            position: crate::domain::relations::types::Position,
            out: &mut dyn RowWriter,
        ) {
            match slot {
                $( Slot::$variant(reader) => (reader.visit)(tree, index, symbols, position, out), )*
            }
        }

        /// Whether the reader in `slot` also looks at the elements the language added for reading.
        fn expands(slot: &Slot) -> Expansions {
            match slot {
                $( Slot::$variant(reader) => reader.expansions, )*
            }
        }

        /// Why the language does not read `relation` from a tree, or nothing when it does or when the
        /// relation is not one of the trees'.
        #[must_use]
        pub fn absence(language: &dyn ParseUseCase, relation: Relation) -> Option<Reason> {
            match relation {
                $( Relation::$variant => match language.$module() {
                    Support(Answer::Reads(_)) => None,
                    Support(Answer::NotInThisLanguage(why)) => Some(why),
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
pub fn kinds_read(language: &dyn ParseUseCase) -> Vec<KindBits> {
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
pub fn build(
    language: &dyn ParseUseCase,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
) -> Result<(), ParseError> {
    if language.recognizes(&request) == Recognition::Declines {
        return Err(ParseError::Unsupported);
    }
    let mut builder = table_builder(table, symbols, request.bytes);
    language.read_tree(&request, &mut builder)?;
    language.complete(
        &request,
        &mut completion(table, symbols, request.bytes, request.file),
    );
    Ok(())
}

/// Does only what the language's library does for the file the request names, through a builder that builds nothing: no table, no
/// name, no row. The allocation row is that reading a file allocates no more than this, so the bound is the language's
/// `read_tree` itself, never a body written beside it.
pub fn baseline(language: &dyn ParseUseCase, request: &ParseRequest<'_>) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = discarding_builder(&mut elements, &mut symbols, request.bytes);
    let _ = language.read_tree(request, &mut builder);
}

/// Reads one file: the language builds its tree, and the rows of the wanted relations are written into `out`.
///
/// # Errors
///
/// What the language's `build` answers.
pub fn read(
    language: &dyn ParseUseCase,
    request: ParseRequest<'_>,
    table: &mut Elements,
    symbols: &mut Symbols,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) -> Result<(), ParseError> {
    build(language, request, table, symbols)?;
    walk(
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
/// is given, in order, to the readers that look at its kind. The table is only read.
pub fn walk(
    language: &dyn ParseUseCase,
    elements: &Elements,
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    file: FileId,
    wanted: &[Relation],
    out: &mut dyn RowWriter,
) {
    let mut slots: [Option<Slot>; COUNT] = [None; COUNT];
    let mut masks: [Mask; KIND_LIMIT] = [[0; WORDS]; KIND_LIMIT];
    let mut reads_any = false;
    for relation in wanted {
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
    let tree = Tree {
        elements,
        source,
        file,
    };
    let indexes: [Option<Index>; COUNT] =
        std::array::from_fn(|place| slots[place].as_ref().map(|each| indexed(each, &tree)));
    // Everything below this line reads the table once; nothing scans it again.
    for position in positions(elements) {
        let Some(mask) = masks.get(usize::from(elements.kind[position].0)) else {
            continue;
        };
        for (word, bits) in mask.iter().enumerate() {
            let mut bits = *bits;
            while bits != 0 {
                let place =
                    word * 64 + usize::try_from(bits.trailing_zeros()).unwrap_or(usize::MAX);
                bits &= bits - 1;
                if let (Some(Some(each)), Some(Some(index))) =
                    (slots.get(place), indexes.get(place))
                {
                    visit(each, &tree, &**index, symbols, position, out);
                }
            }
        }
    }
    // The elements the language added for reading go to the readers that ask for them.
    if slots
        .iter()
        .flatten()
        .any(|each| expands(each) == Expansions::Included)
    {
        for position in expanded_positions(elements) {
            let Some(mask) = masks.get(usize::from(elements.kind[position].0)) else {
                continue;
            };
            for (word, bits) in mask.iter().enumerate() {
                let mut bits = *bits;
                while bits != 0 {
                    let place =
                        word * 64 + usize::try_from(bits.trailing_zeros()).unwrap_or(usize::MAX);
                    bits &= bits - 1;
                    if let (Some(Some(each)), Some(Some(index))) =
                        (slots.get(place), indexes.get(place))
                        && expands(each) == Expansions::Included
                    {
                        visit(each, &tree, &**index, symbols, position, out);
                    }
                }
            }
        }
    }
}
