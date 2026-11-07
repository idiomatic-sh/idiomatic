//! The rendering of a source name reads the injected corpus, and an unreadable source invents no text.
use crate::application::cross_file::rendering::service::Rendering;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Name;
use ryo_relations::domain::relations::types::NamePlace;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Origin;
use ryo_relations::domain::relations::types::PlaceNumber;
use ryo_relations::domain::relations::types::TextHash;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileBytes;

pub(super) fn name(hash: u64, place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(place),
    })
}

pub(super) fn source_text(bytes: &[u8], fails: bool) -> String {
    use crate::outbound::corpus::fixtures::fake::CorpusFake;
    use ryo_values::domain::files::types::CorpusFile;
    use ryo_values::domain::files::types::{FileSize, Nanoseconds};
    let corpus = CorpusFake {
        files: if fails {
            Vec::new()
        } else {
            vec![CorpusFile {
                executable: Default::default(),
                path: FilePath("source".into()),
                bytes: FileBytes(bytes.to_vec()),
            }]
        },
        fails,
        reads: std::sync::Arc::default(),
        outside: Vec::new(),
    };
    let service = Rendering::new(std::sync::Arc::new(corpus));
    let files = [CorpusEntry {
        executable: Default::default(),
        path: FilePath("source".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    }]
    .map(ryo_values::domain::files::types::CorpusInput::Available);
    let mut symbols = Symbols::default();
    places_mut(&mut symbols).push(NamePlace {
        origin: Origin::Source,
        file: FileId(0),
        start: Offset(0),
        end: Offset(3),
    });
    let resolving = crate::domain::finish::types::Resolving {
        unknown: &[],
        paths: &[],
        arenas: crate::domain::finish::types::Arenas(&[]),
        cross: &symbols,
    };
    let mut text = ryo_values::domain::row_texts::types::RenderedText::default();
    let shown = service.show_value(
        &mut text,
        name(1, 0),
        &files,
        &resolving,
        &mut crate::domain::finish::types::Loaded::default(),
    );
    match shown.map(|kept| {
        ryo_values::domain::records::service::lent(
            ryo_values::domain::row_texts::types::RowsTextView(&text),
            kept,
        )
    }) {
        Some(ryo_values::domain::row_texts::types::Cell::Text(piece)) => piece.0.to_owned(),
        Some(
            ryo_values::domain::row_texts::types::Cell::File(_)
            | ryo_values::domain::row_texts::types::Cell::Number(_)
            | ryo_values::domain::row_texts::types::Cell::Span(_),
        )
        | None => String::new(),
    }
}
