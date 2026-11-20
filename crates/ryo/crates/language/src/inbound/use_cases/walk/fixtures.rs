//! The test support of the `walk` use case: the fillers concept's fixture language walked over a file of words (`*.walk`), one node
//! over one token per word, a diagnostic for each `!`. A file whose bytes are `FAILING` makes its parser fail.

use super::errors::WalkError;
use super::interface::WalkUseCase;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::facts::types::FileFacts;
use crate::domain::fillers::fixtures::{Walking, WalkingFile};
use crate::domain::fillers::service::file_source;
use crate::domain::node_table::service::{diagnose, enter_node, finished, leave_node, token};
use crate::domain::node_table::types::{FieldCode, NodeTableBuilder};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;

/// The bytes the fixture's parser fails on.
pub const FAILING: &[u8] = b"\xff";

/// The kind of the fixture's one node.
const SENTENCE: KindBits = KindBits(1);

/// The kind of a word.
pub const WORD: KindBits = KindBits(2);

impl WalkUseCase for Walking {
    type Scratch = ();
    const KIND_COUNT: Count = Count(3);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if request.source.0 == FAILING {
            return Err(WalkError::Library);
        }
        if !request.path.0.ends_with(".walk") {
            return Err(WalkError::Unsupported);
        }
        let bytes = request.source.0;
        let offset = |at: usize| Offset(u32::try_from(at).unwrap_or(u32::MAX));
        enter_node(builder, SENTENCE, FieldCode(0), Offset(0));
        let mut start = None;
        for (at, byte) in bytes.iter().chain(std::iter::once(&b' ')).enumerate() {
            match (start, byte.is_ascii_whitespace()) {
                (None, false) => start = Some(at),
                (Some(from), true) => {
                    token(builder, WORD, FieldCode(0), offset(from), offset(at));
                    start = None;
                }
                _ => {}
            }
            if *byte == b'!' {
                diagnose(
                    builder,
                    offset(at),
                    offset(at.saturating_add(1)),
                    NameText("an exclamation"),
                );
            }
        }
        leave_node(builder, offset(bytes.len()));
        let table = finished(builder);
        let mut file = WalkingFile {
            table,
            source: request.source.0,
            file: request.file,
            fills: Count(0),
        };
        let mut source = file_source::<Self>(&mut file);
        visit.visit(FileFacts {
            table,
            source: request.source,
            file: request.file,
            facts: &mut source,
        });
        Ok(())
    }
}
