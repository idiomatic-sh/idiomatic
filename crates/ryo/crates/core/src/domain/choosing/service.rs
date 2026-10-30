//! The `choosing` service: which rows of a file's findings are written, with no IO and no ports.

use crate::domain::render::service::{finding_parts, row_cells};
use crate::domain::render::types::Rendered;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::service::lent;
use ryo_values::domain::records::types::{Cell, RowsTextView, TextRange};

/// The rows of a file's findings to write, in the order they leave in: those laid out as a finding's, and, when `only` names files,
/// those whose subject is a path it names.
pub(crate) fn chosen<'a>(
    rendered: &'a Rendered,
    only: &'a [FilePath],
) -> impl Iterator<Item = &'a [Option<Cell<TextRange>>]> {
    let text = RowsTextView(&rendered.text.0);
    rendered
        .order
        .iter()
        .map(move |each| row_cells(&rendered.cells, &rendered.ends, each.row))
        .filter(move |cells| {
            finding_parts(cells).is_some_and(|found| {
                only.is_empty()
                    || only.iter().any(|asked| {
                        matches!(
                            found.subject.map(|kept| lent(text, kept)),
                            Some(Cell::File(path) | Cell::Text(path)) if path.0 == &*asked.0
                        )
                    })
            })
        })
}
