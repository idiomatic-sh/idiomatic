//! The types of the `fillers` concept: the bundle a language implements for the engine, one filler per `Filled` relation of the
//! catalog, the `fills!` item that writes one, and the fact source the engine reads them through.

use crate::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FilledColumns, FillerSentence, PartialRows,
};
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{FileId, Missing, Relation, ReportText, Rows};

/// What a language's filler of a relation did for a file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::types::Fill;
/// use ryo_relations::domain::answers::types::AbsenceCause;
///
/// assert_ne!(Fill::Written, Fill::Absent(AbsenceCause::NotYetRead));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// It wrote every row it could (a part it could not read it said through `Rows::partial`).
    Written,
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

/// What a language's fillers are made of: one associated function for each relation the catalog declares `Filled`, with no
/// default body, so a relation added to the catalog stops every language from compiling until it fills it (calling its `fills!`
/// item) or answers `Fill::Absent`; a `NodeTable` relation is read in place and has none. Generated from the catalog's one list.
/// It holds the language's per-file view, which the walk lends and every filler reads: the walk contract (`WalkUseCase`) extends it.
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
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, columns $columns:tt
    ) => {
        /// Fills the relation of its name for the file, or answers why it is absent.
        fn $module(
            file: &mut Self::File<'_>,
            words: &mut $crate::domain::facts::types::FileWords,
            rows: &mut dyn ::ryo_relations::domain::relations::types::Rows<
                ::ryo_relations::domain::relations::collections::relations::$module::item::$row,
            >,
        ) -> $crate::domain::fillers::types::Fill;
    };
}

/// The `Fillers` functions of a language that has none of the relations: each `Filled` relation answers `Absent(NoSuchConstruct)`.
/// What the language that reads nothing implements, inside its `impl Fillers`.
#[doc(hidden)]
#[macro_export]
macro_rules! no_fillers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        $( $( ::ryo_relations::$module!($crate::no_filler, $module); )* )*
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
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, columns $columns:tt
    ) => {
        fn $module(
            _file: &mut Self::File<'_>,
            _words: &mut $crate::domain::facts::types::FileWords,
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

/// The rows a filler writes, kept column by column as words, and the first part it said it could not read.
pub(crate) struct ColumnsWriter<'o, R> {
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
        for (column, number) in self.out.0.iter_mut().zip(words.as_ref()) {
            column.0.push(CellWord(number.0));
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
        impl<W: Fillers> FactSource for FileSource<'_, '_, W> {
            fn fill(&mut self, relation: Relation, words: &mut FileWords, out: &mut FilledColumns) -> FactAnswer {
                let file = &mut *self.file;
                $( $( ::ryo_relations::$module!($crate::fill_arm, $module, W, file, relation, words, out); )* )*
                FactAnswer::Refused(AbsenceCause::NoSuchConstruct)
            }

            fn render(&mut self, request: RenderRequest) -> RenderOutcome {
                W::render(self.file, request)
            }
        }
    };
}

ryo_relations::each_relation!(file_source_answers);

/// One arm of a file source's `fill`, from a relation's item declaration: a `Filled` relation is filled by the language's function
/// of it into one column per column of the relation.
#[doc(hidden)]
#[macro_export]
macro_rules! fill_arm {
    (
        $module:ident, $walk:ident, $file:ident, $relation:ident, $words:ident, $out:ident
        item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt
    ) => {};
    (
        $module:ident, $walk:ident, $file:ident, $relation:ident, $words:ident, $out:ident
        item $item:ident, row $row:tt, column $column:tt, expected $expected:tt, class $class:ident,
        scope $scope:ident, notes $notes:ident, form Filled $key:tt, columns $columns:tt
    ) => {
        if $relation == ::ryo_relations::domain::relations::types::Relation::$item {
            type Item =
                ::ryo_relations::domain::relations::collections::relations::$module::item::$item;
            let width = <Item as ::ryo_relations::domain::relations::collections::relations::interface::RelationsItem>::kinds().len();
            $out.0.truncate(width);
            for column in &mut $out.0 {
                column.0.clear();
            }
            $out.0.resize_with(width, $crate::domain::facts::types::FilledColumn::default);
            let mut writer = $crate::domain::fillers::types::ColumnsWriter::<Item> {
                out: $out,
                partial: None,
                relation: ::std::marker::PhantomData,
            };
            return match $walk::$module($file, $words, &mut writer) {
                $crate::domain::fillers::types::Fill::Absent(cause) => {
                    writer.out.0.clear();
                    $crate::domain::facts::types::FactAnswer::Refused(cause)
                }
                $crate::domain::fillers::types::Fill::Written => writer.partial.map_or(
                    $crate::domain::facts::types::FactAnswer::Answered,
                    $crate::domain::facts::types::FactAnswer::Partial,
                ),
            };
        }
    };
}
