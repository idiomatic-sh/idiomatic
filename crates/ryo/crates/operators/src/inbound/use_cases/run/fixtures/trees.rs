//! The resources of the files whose trees the cases read: a file of three tokens, a file of a node of three tokens and a
//! token, and a file of only its root.
use super::TextResources;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_field, name_kind, write_fields};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::constants::NO_FIELD;
use ryo_relations::domain::relations::types::{FieldId, FileId, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;

/// The role of the first child of each parent of the nested tree.
const HEAD: FieldId = FieldId(1);

impl TextResources {
    /// The resources of the file `abc` in file 0: a `file` node over the tokens `a` (a `word`), `b` (an `other`) and `c` (a
    /// `word`), at positions 1 to 3.
    #[must_use]
    pub fn small_tree() -> Self {
        let source = b"abc".to_vec();
        let (mut tree, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder = table_builder(&mut tree, &mut symbols, ByteSlice(&source));
        enter_node(&mut builder, KindBits(1), Offset(0));
        for (bits, start) in [(2, 0), (3, 1), (2, 2)] {
            token(
                &mut builder,
                KindBits(bits),
                Offset(start),
                Offset(start + 1),
            );
        }
        leave_node(&mut builder, Offset(3));
        for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
            let mut names = completion(&mut tree, &mut symbols, ByteSlice(&source), FileId(0));
            name_kind(&mut names, KindBits(bits), NameText(name));
        }
        Self {
            source,
            symbols,
            tree,
            renderings: Vec::new(),
            calls: Vec::new(),
        }
    }

    /// The resources of the file `abcd` in file 0: a `file` node over an `other` node over the tokens `a`, `b` and `c` (each
    /// a `word`), at positions 1 to 4, then the token `d` (a `word`) at position 5. The first child of each parent (the
    /// `other` node and `a`) has the role `head`.
    #[must_use]
    pub fn nested_tree() -> Self {
        let source = b"abcd".to_vec();
        let (mut tree, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder = table_builder(&mut tree, &mut symbols, ByteSlice(&source));
        enter_node(&mut builder, KindBits(1), Offset(0));
        enter_node(&mut builder, KindBits(3), Offset(0));
        for start in 0..3 {
            token(&mut builder, KindBits(2), Offset(start), Offset(start + 1));
        }
        leave_node(&mut builder, Offset(3));
        token(&mut builder, KindBits(2), Offset(3), Offset(4));
        leave_node(&mut builder, Offset(4));
        for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
            let mut names = completion(&mut tree, &mut symbols, ByteSlice(&source), FileId(0));
            name_kind(&mut names, KindBits(bits), NameText(name));
        }
        // The first child of each parent, the `other` node and the token `a`, has the role `head`; the others have none.
        let mut roles = completion(&mut tree, &mut symbols, ByteSlice(&source), FileId(0));
        write_fields(&mut roles, |_, at| {
            if matches!(at.0, 1 | 2) {
                HEAD
            } else {
                NO_FIELD
            }
        });
        name_field(&mut roles, HEAD, NameText("head"));
        Self {
            source,
            symbols,
            tree,
            renderings: Vec::new(),
            calls: Vec::new(),
        }
    }

    /// The resources of the empty file in file 0: a `file` node alone, the tree's root, with no edge.
    #[must_use]
    pub fn root_only() -> Self {
        let source = Vec::new();
        let (mut tree, mut symbols) = (Elements::default(), Symbols::default());
        let mut builder = table_builder(&mut tree, &mut symbols, ByteSlice(&source));
        enter_node(&mut builder, KindBits(1), Offset(0));
        leave_node(&mut builder, Offset(0));
        let mut names = completion(&mut tree, &mut symbols, ByteSlice(&source), FileId(0));
        name_kind(&mut names, KindBits(1), NameText("file"));
        Self {
            source,
            symbols,
            tree,
            renderings: Vec::new(),
            calls: Vec::new(),
        }
    }
}
