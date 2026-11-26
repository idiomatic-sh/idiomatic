//! The numbered row of the language contract: the linked provider hands out a numbered visit, and a Go file read through it is lent as a numbered tree.

fn main() {
    use ryo::languages::{self, NumberedVisit};

    let language = ryo_go::linked();
    let engine = language.parse_engine();
    let relations = engine
        .file_relations()
        .expect("the provider declares file relations");
    let visit: &dyn NumberedVisit = relations.numbered();
    let mut reader = visit.open();
    let mut symbols = languages::Symbols::default();
    let mut lent = 0usize;
    reader
        .visit(
            languages::ParseRequest {
                path: languages::SourcePath("main.go"),
                file: languages::FileId(0),
                bytes: languages::ByteSlice(b"package main\n\nfunc main() {}\n"),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Unwanted,
            },
            &[],
            &[],
            None,
            &mut symbols,
            &mut |_, _| lent += 1,
        )
        .expect("a Go file is lent numbered");
    assert_eq!(lent, 1, "the visit lends the file once");
}
