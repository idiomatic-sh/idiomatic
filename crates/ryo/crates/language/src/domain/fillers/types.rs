//! The types of the `fillers` concept: the bundle a language implements for the engine, one filler per `Filled` relation of the
//! catalog, the `fills!` item that writes one (whole, and by key for the columns it has a body for), the names a language states
//! for the whole run, and the fact source the engine reads the fillers through.

use crate::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FillExtent, FilledColumns, FillerSentence,
    PartialRows, Wanted,
};
use crate::domain::fixed_names::types::FixedNames;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{
    Column, FileId, Missing, Relation, ReportText, Rows,
};
use ryo_relations::domain::terms::types::NameText;

/// What a language's filler of a relation did for a file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FillExtent;
/// use ryo_language::domain::fillers::types::Fill;
/// use ryo_relations::domain::answers::types::AbsenceCause;
///
/// assert_ne!(Fill::Written(FillExtent::Asked), Fill::Absent(AbsenceCause::NotYetRead));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// It wrote every row it could of what it says it wrote: the rows asked for, or the whole relation where a key was asked and
    /// it has no body for that key (a part it could not read it said through `Rows::partial`).
    Written(FillExtent),
    /// The relation is not in the language, for this cause; it wrote nothing.
    Absent(AbsenceCause),
}

/// One `fills!` item: the relation it fills, from which language's per-file view, and how. Only `ryo::fills!` implements it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::service::filled;
/// use ryo_language::domain::fillers::fixtures::{SyntaxErrorFill, Walking};
///
/// let _fill = filled::<SyntaxErrorFill>;
/// let _language = Walking;
/// ```
pub trait FillItem {
    /// The language whose per-file view it reads.
    type Walk: Fillers;
    /// The relation it fills.
    type Relation: RelationsItem;
    /// Writes every row of the relation for the file from the language's view, interning the texts it builds into `words`.
    fn fill(
        file: &mut <Self::Walk as Fillers>::File<'_>,
        words: &mut FileWords,
        rows: &mut dyn Rows<<Self::Relation as RelationsItem>::Row>,
    );
}

/// One `fills!` item with a key clause: it also writes the rows of one key, for the columns it names. Only `ryo::fills!`
/// implements it. A language that fills a relation has a body for every column the relation's catalog item declares among its
/// `keys`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::fixtures::TokenLinkFill;
/// use ryo_language::domain::fillers::types::KeyedFillItem;
/// use ryo_relations::domain::catalog::service::key_columns;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(<TokenLinkFill as KeyedFillItem>::KEYS, key_columns(Relation::TokenLink));
/// ```
pub trait KeyedFillItem: FillItem {
    /// The columns it has a body for.
    const KEYS: &'static [Column];
    /// Writes the rows of the relation whose `column` holds `key`, and no other row, from the language's view. `column` is one
    /// of `KEYS`; a word key is read with `facts::service::text`.
    fn fill_key(
        file: &mut <Self::Walk as Fillers>::File<'_>,
        words: &mut FileWords,
        column: Column,
        key: CellWord,
        rows: &mut dyn Rows<<Self::Relation as RelationsItem>::Row>,
    );
}

/// The names of the kinds a language's tables hold, each once: a kind's code is its place.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::types::WalkKinds;
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(WalkKinds(vec![NameText("blank"), NameText("sentence")]).0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WalkKinds(pub Vec<NameText<'static>>);

/// The names of the fields a language's nodes have, each once: a field's code is its place, and place 0 is "no field".
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::types::FieldNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(FieldNames(vec![NameText(""), NameText("label")]).0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldNames(pub Vec<NameText<'static>>);

/// What a language names for the whole run: a kind's code, a field's code and a fixed word are places in these lists.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::fixtures::Walking;
/// use ryo_language::domain::fillers::types::Fillers;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let names = <Walking as Fillers>::vocabulary();
/// assert_eq!(names.kinds.0.get(2), Some(&NameText("word")));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WalkVocabulary {
    /// The kinds: a kind's code is its place.
    pub kinds: WalkKinds,
    /// The fields: a field's code is its place, and place 0 is "no field".
    pub fields: FieldNames,
    /// The fixed names: a `FixedWord` is a place here.
    pub fixed: FixedNames,
}

/// What a language's fillers are made of: one associated function for each relation the catalog declares `Filled`, with no
/// default body, so a relation added to the catalog stops every language from compiling until it fills it (calling its `fills!`
/// item) or answers `Fill::Absent`; a `NodeTable` relation is read in place and has none. Generated from the catalog's one list.
/// Each is asked for the whole relation or for the rows of one key (`Wanted`), and says which it wrote. It holds the language's
/// per-file view, which the walk lends and every filler reads: the walk contract (`WalkUseCase`) extends it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::types::Fillers;
/// use ryo_language::domain::fillers::fixtures::Walking;
///
/// fn fills<W: Fillers>(_: &W) {}
/// fills(&Walking);
/// ```
pub trait Fillers: Send + Sync + 'static {
    /// The language's per-file view: its parse in place, a typed node per id, its model built on first need. Named by `fills!`.
    type File<'f>: Send
    where
        Self: 'f;

    ryo_relations::each_relation!(filler_methods);

    /// The source form of a node of the file in a syntax role, from the language's per-file view.
    fn render(file: &mut Self::File<'_>, request: RenderRequest) -> RenderOutcome;

    /// The names of the run: the language's kinds, fields and fixed names, each by place.
    fn vocabulary() -> WalkVocabulary;
}

/// The methods of `Fillers`: each relation's item declaration handed to `filler_method` with its module.
macro_rules! filler_methods {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        $( $( ::ryo_relations::$module!($crate::filler_method, $module); )* )*
    };
}
use filler_methods;

/// The `Fillers` function of one relation, from its item declaration: one for a `Filled` relation, none for a `NodeTable` relation,
/// which no filler writes.
#[doc(hidden)]
#[macro_export]
macro_rules! filler_method {
    (
        $module:ident item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt
    ) => {};
    (
        $module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt,
        columns $columns:tt
    ) => {
        /// Appends the wanted rows of the relation of its name for the file, and says which it wrote, or answers why the
        /// relation is absent.
        fn $module(
            file: &mut Self::File<'_>,
            words: &mut $crate::domain::facts::types::FileWords,
            wanted: $crate::domain::facts::types::Wanted,
            rows: &mut dyn ::ryo_relations::domain::relations::types::Rows<
                ::ryo_relations::domain::relations::collections::relations::$module::item::$row,
            >,
        ) -> $crate::domain::fillers::types::Fill;
    };
}

/// The `Fillers` functions of a language that has none of the relations: each `Filled` relation answers `Absent(NoSuchConstruct)`,
/// and its vocabulary names nothing. What the language that reads nothing implements, inside its `impl Fillers`.
#[doc(hidden)]
#[macro_export]
macro_rules! no_fillers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        $( $( ::ryo_relations::$module!($crate::no_filler, $module); )* )*

        /// A language that has none of the relations names nothing.
        fn vocabulary() -> $crate::domain::fillers::types::WalkVocabulary {
            $crate::domain::fillers::types::WalkVocabulary::default()
        }
    };
}

/// One relation's function of `no_fillers`, from its item declaration: none for a `NodeTable` relation.
#[doc(hidden)]
#[macro_export]
macro_rules! no_filler {
    (
        $module:ident item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt
    ) => {};
    (
        $module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt,
        columns $columns:tt
    ) => {
        fn $module(
            _file: &mut Self::File<'_>,
            _words: &mut $crate::domain::facts::types::FileWords,
            _wanted: $crate::domain::facts::types::Wanted,
            _rows: &mut dyn ::ryo_relations::domain::relations::types::Rows<
                ::ryo_relations::domain::relations::collections::relations::$module::item::$row,
            >,
        ) -> $crate::domain::fillers::types::Fill {
            $crate::domain::fillers::types::Fill::Absent(
                ::ryo_relations::domain::answers::types::AbsenceCause::NoSuchConstruct,
            )
        }
    };
}

/// The rows a filler writes, appended column by column as words, and the first part it said it could not read.
pub(crate) struct ColumnsWriter<'o, R> {
    /// Selected catalog places; an empty set still counts rows.
    pub(crate) mask: &'o crate::domain::facts::projection::ColumnProjection,
    pub(crate) written: usize,
    /// Where the columns go.
    pub(crate) out: &'o mut FilledColumns,
    /// The first part the filler could not read.
    pub(crate) partial: Option<PartialRows>,
    /// The relation written.
    pub(crate) relation: std::marker::PhantomData<R>,
}

impl<R: RelationsItem> Rows<R::Row> for ColumnsWriter<'_, R> {
    fn write(&mut self, row: R::Row) {
        let words = R::words(row);
        self.written += 1;
        if matches!(self.mask, crate::domain::facts::projection::ColumnProjection::All) {
            for (column, number) in self.out.0.iter_mut().zip(words.as_ref()) {
                column.0.push(CellWord(number.0));
            }
        } else {
            let words = words.as_ref();
            for (part, chunk) in words.chunks(64).enumerate() {
                let mut selected = self.mask.word(part);
                while selected != 0 {
                    let i = selected.trailing_zeros() as usize;
                    if i >= chunk.len() { break; }
                    self.out.0[part * 64 + i].0.push(CellWord(chunk[i].0));
                    selected &= selected - 1;
                }
            }
        }
    }

    fn partial(&mut self, cause: ReportText<'_>, missing: Missing) {
        if self.partial.is_none() {
            self.partial = Some(PartialRows {
                cause: FillerSentence(cause.0.into()),
                missing,
            });
        }
    }

    fn partial_at(&mut self, _file: FileId, cause: ReportText<'_>, missing: Missing) {
        <Self as Rows<R::Row>>::partial(self, cause, missing);
    }
}

/// The engine's view of one file of a language: its fillers over its per-file view.
pub(crate) struct FileSource<'a, 'f, W: Fillers> {
    /// Its view of the file, lent for the visit.
    pub(crate) file: &'a mut W::File<'f>,
}

/// The fact source of a language's file, generated from the catalog: each `Filled` relation answered by its `Fillers` function
/// through `fill_arm`; a `NodeTable` relation has none and is refused.
macro_rules! file_source_answers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        impl<W: Fillers> FileSource<'_, '_, W> {
            /// The fill of a relation by the language's filler of it.
            fn answered(
                &mut self,
                mask: &$crate::domain::facts::projection::ColumnProjection,
                written: &mut usize,
                relation: Relation,
                wanted: Wanted,
                words: &mut FileWords,
                out: &mut FilledColumns,
            ) -> FactAnswer {
                let file = &mut *self.file;
                $( $( ::ryo_relations::$module!($crate::fill_arm, $module, W, file, relation, wanted, words, out, mask, written); )* )*
                FactAnswer::Refused(AbsenceCause::NoSuchConstruct)
            }
        }

        impl<W: Fillers> FactSource for FileSource<'_, '_, W> {
            fn fill(
                &mut self,
                relation: Relation,
                wanted: Wanted,
                words: &mut FileWords,
                out: &mut FilledColumns,
            ) -> FactAnswer {
                // SPIKE: under RYO_X_FILLS each fill is counted (`fillers::service::fill_counted`); else nothing is read.
                let Some(begun) = $crate::domain::fillers::service::fill_begun() else {
                    return self.answered(&$crate::domain::facts::projection::ColumnProjection::All, &mut 0, relation, wanted, words, out);
                };
                let held = out.0.first().map_or(0, |column| column.0.len());
                let answer = self.answered(&$crate::domain::facts::projection::ColumnProjection::All, &mut 0, relation, wanted, words, out);
                let rows = out.0.first().map_or(0, |column| column.0.len()).saturating_sub(held);
                $crate::domain::fillers::service::fill_counted(relation, wanted, rows, begun);
                answer
            }

            fn fill_columns(
                &mut self, relation: Relation, wanted: Wanted, mask: &$crate::domain::facts::projection::ColumnProjection,
                words: &mut FileWords, out: &mut FilledColumns,
            ) -> (FactAnswer, usize) {
                out.clear_for(relation);
                let begun = $crate::domain::fillers::service::fill_begun();
                let mut rows = 0;
                let answer = self.answered(mask, &mut rows, relation, wanted, words, out);
                if let Some(begun) = begun {
                    $crate::domain::fillers::service::fill_counted(relation, wanted, rows, begun);
                }
                (answer, rows)
            }

            fn render(&mut self, request: RenderRequest) -> RenderOutcome {
                W::render(self.file, request)
            }
        }
    };
}

ryo_relations::each_relation!(file_source_answers);

/// One arm of a file source's `fill`, from a relation's item declaration: a `Filled` relation is filled by the language's function
/// of it, which appends the wanted rows to one column per column of the relation. A fill never clears what an earlier fill of the
/// file wrote; a relation the language answers as absent leaves the columns as they were.
#[doc(hidden)]
#[macro_export]
macro_rules! fill_arm {
    (
        $module:ident, $walk:ident, $file:ident, $relation:ident, $wanted:ident, $words:ident, $out:ident, $mask:ident, $written:ident
        item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt
    ) => {};
    (
        $module:ident, $walk:ident, $file:ident, $relation:ident, $wanted:ident, $words:ident, $out:ident, $mask:ident, $written:ident
        item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt, columns $columns:tt
    ) => {
        if $relation == ::ryo_relations::domain::relations::types::Relation::$item {
            type Item =
                ::ryo_relations::domain::relations::collections::relations::$module::item::$item;
            let width = <Item as ::ryo_relations::domain::relations::collections::relations::interface::RelationsItem>::kinds().len();
            // What the columns held before this fill: an absent relation leaves them so.
            let held_columns = $out.0.len();
            if held_columns < width {
                $out.0.resize_with(width, $crate::domain::facts::types::FilledColumn::default);
            }
            let mut writer = $crate::domain::fillers::types::ColumnsWriter::<Item> {
                out: $out,
                mask: $mask,
                written: 0,
                partial: None,
                relation: ::std::marker::PhantomData,
            };
            let filled = $walk::$module($file, $words, $wanted, &mut writer);
            *$written = writer.written;
            return match filled {
                $crate::domain::fillers::types::Fill::Absent(cause) => {
                    *$written = 0;
                    for (i, column) in writer.out.0.iter_mut().take(width).enumerate() {
                        if $mask.contains(i) {
                            column.0.truncate(column.0.len() - writer.written);
                        }
                    }
                    writer.out.0.truncate(held_columns);
                    $crate::domain::facts::types::FactAnswer::Refused(cause)
                }
                $crate::domain::fillers::types::Fill::Written(extent) => writer.partial.map_or(
                    $crate::domain::facts::types::FactAnswer::Answered(extent),
                    $crate::domain::facts::types::FactAnswer::Partial,
                ),
            };
        }
    };
}
