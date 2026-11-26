//! The numbered row of the language contract: the linked provider hands out a numbered visit, and a Python file read through it is lent as a numbered tree.

#[test]
fn a_python_file_is_lent_numbered() {
    use ryo::languages::{self, NumberedVisit};

    let language = ryo_python::linked();
    let engine = language.parse_engine();
    let visit: &dyn NumberedVisit = engine.file_relations().numbered();
    let mut reader = visit.open();
    let mut symbols = languages::Symbols::default();
    let mut lent = 0usize;
    reader
        .visit(
            languages::ParseRequest {
                path: languages::SourcePath("a.py"),
                file: languages::FileId(0),
                bytes: languages::ByteSlice(b"x = 1\n"),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Unwanted,
            },
            &[],
            &[],
            None,
            &mut symbols,
            &mut |_, _| lent += 1,
        )
        .expect("a Python file is lent numbered");
    assert_eq!(lent, 1, "the visit lends the file once");
}
