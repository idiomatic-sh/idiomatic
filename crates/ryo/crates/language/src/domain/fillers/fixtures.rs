//! The test support of the `fillers` concept: a language over files of words, its per-file view (the finished table, the bytes,
//! the file's number and how many fills it answered), its `syntax-error` filler (only in part when the file holds a `?`), and every
//! other relation said not read yet. The walk use case's fixture walks it.

use crate::domain::facts::service::syntax_error_rows;
use crate::domain::facts::types::FileWords;
use crate::domain::fillers::service::filled;
use crate::domain::fillers::types::{Fill, FillItem, Fillers};
use crate::domain::node_table::types::NodeTable;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::collections::relations::syntax_error::item::{
    SyntaxError, SyntaxErrorRow,
};
use ryo_relations::domain::relations::types::{Count, FileId, Missing, ReportText, Rows};

/// The language that walks files of words.
#[derive(Clone, Copy, Debug)]
pub struct Walking;

/// Its view of one file: the finished table, the file's number and how many fills it answered.
#[derive(Debug)]
pub struct WalkingFile<'f> {
    /// The file's table.
    pub table: &'f NodeTable,
    /// The file's bytes.
    pub source: &'f [u8],
    /// The file's number.
    pub file: FileId,
    /// How many relations it filled.
    pub fills: Count,
}

/// The fixture's `syntax-error` filler.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxErrorFill;

impl FillItem for SyntaxErrorFill {
    type Walk = Walking;
    type Relation = SyntaxError;

    /// Writes the file's diagnostics as `syntax-error` rows, counting the fill.
    fn fill(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        rows: &mut dyn Rows<SyntaxErrorRow>,
    ) {
        file.fills = Count(file.fills.0.saturating_add(1));
        syntax_error_rows(file.table, file.file, words, rows);
        if file.source.contains(&b'?') {
            rows.partial(
                ReportText("a question the fixture cannot read"),
                Missing::Any,
            );
        }
    }
}

/// The fixture's filler of a relation it has not read: absent, for that reason. `syntax_error` is written by hand; a `NodeTable`
/// relation has no filler.
macro_rules! absent_filler {
    (syntax_error $( $rest:tt )*) => {};
    ($module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt) => {};
    ($module:ident item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column $column:tt, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, columns $columns:tt) => {
        fn $module(
            _file: &mut WalkingFile<'_>,
            _words: &mut FileWords,
            _rows: &mut dyn Rows<
                ryo_relations::domain::relations::collections::relations::$module::item::$row,
            >,
        ) -> Fill {
            Fill::Absent(ryo_relations::domain::answers::types::AbsenceCause::NotYetRead)
        }
    };
}

/// Every relation's filler of the fixture.
macro_rules! absent_fillers {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        $( $( ryo_relations::$module!(absent_filler, $module); )* )*
    };
}

impl Fillers for Walking {
    type File<'f> = WalkingFile<'f>;

    ryo_relations::each_relation!(absent_fillers);

    fn syntax_error(
        file: &mut WalkingFile<'_>,
        words: &mut FileWords,
        rows: &mut dyn Rows<SyntaxErrorRow>,
    ) -> Fill {
        filled::<SyntaxErrorFill>(file, words, rows)
    }

    fn render(_file: &mut WalkingFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}
