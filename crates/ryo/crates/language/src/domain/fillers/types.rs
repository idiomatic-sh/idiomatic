//! The types of the `fillers` concept: the bundle a language implements for the engine, one filler per `Filled` relation of the
//! catalog, the `fills!` item that writes one whole, the names a language states for the whole run, and the fact source the
//! engine reads the fillers through: one request a file, each relation asked written into the file's fact set.

use crate::domain::fact_tables::service::{asked, partial, refused, row};
use crate::domain::fact_tables::types::{FactCells, FactRequest, FactSet, FactWriter};
use crate::domain::facts::types::{CellWord, FactSource, FileWords, FillerSentence, PartialRows};
use crate::domain::fixed_names::types::FixedNames;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{FileId, Missing, Relation, ReportText, Rows};
use ryo_relations::domain::terms::types::NameText;

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
    /// It wrote every row of the relation it could (a part it could not read it said through `Rows::partial`).
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
        $( spells $spells:tt, )? columns $columns:tt
    ) => {
        /// Writes every row of the relation of its name for the file, or answers why the relation is absent.
        fn $module(
            file: &mut Self::File<'_>,
            words: &mut $crate::domain::facts::types::FileWords,
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
        $( spells $spells:tt, )? columns $columns:tt
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

/// The rows a filler writes, each written into its relation's table of the file's set as cells, and the first part it said
/// it could not read.
pub(crate) struct TableRows<'w, 's, R> {
    /// Where the rows go.
    pub(crate) writer: &'w mut FactWriter<'s>,
    /// One row's cells, kept from row to row.
    pub(crate) cells: Vec<CellWord>,
    /// The relation written.
    pub(crate) relation: std::marker::PhantomData<R>,
}

impl<R: RelationsItem> Rows<R::Row> for TableRows<'_, '_, R> {
    fn write(&mut self, written: R::Row) {
        let words = R::words(written);
        self.cells.clear();
        self.cells
            .extend(words.as_ref().iter().map(|number| CellWord(number.0)));
        row(self.writer, FactCells(&self.cells));
    }

    fn partial(&mut self, cause: ReportText<'_>, missing: Missing) {
        partial(
            self.writer,
            PartialRows {
                cause: FillerSentence(cause.0.into()),
                missing,
            },
        );
    }

    fn partial_at(&mut self, _file: FileId, cause: ReportText<'_>, missing: Missing) {
        <Self as Rows<R::Row>>::partial(self, cause, missing);
    }
}

/// The fact source of one file over a language's fillers: what `fillers::service::file_source` lends the engine.
pub(crate) struct FileSource<'a, 'f, W: Fillers> {
    /// The language's per-file view.
    pub(crate) file: &'a mut W::File<'f>,
}

/// The fact source's answer: each relation asked, by its filler.
macro_rules! file_source_answers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        impl<W: Fillers> FileSource<'_, '_, W> {
            /// One relation of the request, by the language's filler of it: written, or refused for its cause.
            fn answered(&mut self, relation: Relation, words: &mut FileWords, out: &mut FactSet) {
                let file = &mut *self.file;
                $( $( ::ryo_relations::$module!($crate::fill_arm, $module, W, file, relation, words, out); )* )*
                refused(out, relation, AbsenceCause::NoSuchConstruct);
            }
        }

        impl<W: Fillers> FactSource for FileSource<'_, '_, W> {
            fn facts(&mut self, request: &FactRequest, words: &mut FileWords, out: &mut FactSet) {
                for relation in asked(request) {
                    // SPIKE: under RYO_X_FILLS each relation's fill is counted (`fillers::service::fill_counted`).
                    let Some(begun) = $crate::domain::fillers::service::fill_begun() else {
                        self.answered(relation, words, out);
                        continue;
                    };
                    self.answered(relation, words, out);
                    let rows = $crate::domain::fact_tables::reading::service::every_row(
                        $crate::domain::fact_tables::service::table_of(out, relation),
                    );
                    $crate::domain::fillers::service::fill_counted(
                        relation,
                        usize::try_from(rows.end.0).unwrap_or(usize::MAX),
                        begun,
                    );
                }
            }

            fn render(&mut self, request: RenderRequest) -> RenderOutcome {
                W::render(self.file, request)
            }
        }
    };
}

ryo_relations::each_relation!(file_source_answers);

/// One relation's arm of the fact source's answer: its filler run into a writer of its table, when it is the relation asked.
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
        scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt, $( spells $spells:tt, )? columns $columns:tt
    ) => {
        if $relation == ::ryo_relations::domain::relations::types::Relation::$item {
            type Item =
                ::ryo_relations::domain::relations::collections::relations::$module::item::$item;
            let mut writing = $crate::domain::fact_tables::service::writer($out, $relation);
            let mut rows = $crate::domain::fillers::types::TableRows::<Item> {
                writer: &mut writing,
                cells: ::std::vec::Vec::new(),
                relation: ::std::marker::PhantomData,
            };
            match $walk::$module($file, $words, &mut rows) {
                $crate::domain::fillers::types::Fill::Written => {
                    $crate::domain::fact_tables::service::finished(writing);
                }
                $crate::domain::fillers::types::Fill::Absent(cause) => {
                    drop(writing);
                    $crate::domain::fact_tables::service::refused($out, $relation, cause);
                }
            }
            return;
        }
    };
}
