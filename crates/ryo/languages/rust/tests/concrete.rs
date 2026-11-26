//! The concrete row of the language contract: the elements the engine reads as `syntax-tree` and `syntax-child` of a Rust file are the
//! file's own tree, as the table the linter read before the walk held them: what a macro call's arguments expand to is no row of those
//! relations, though it is a position of the lent tree (`includes_expansions` says so for the relations that do cover it). The lent
//! tree's own part, below `own_end`, equals the old table element for element; the lent tree also holds the expansions after it.
//!
//! This is the acceptance row of the adapter reading a relation below `own_end` when the language says it does not include expansions:
//! the clippy cases and `rustcases` show it end to end.

use ryo::languages::{self, NumberedVisit, node_table as t};

const SOURCES: &[&str] = &[
    "pub fn f() -> Vec<u32> {\n    vec![0xfF]\n}\n",
    "pub fn f() -> String {\n    println!(\"a\u{200B}b\");\n    assert_eq!(\"a\", \"a\u{200B}b\");\n    format!(\"{}\", \"a\u{200B}b\")\n}\n",
    "pub fn f() -> String {\n    format!(\"{}\", vec!(1u8).len())\n}\npub fn g() -> Vec<Vec<u8>> {\n    vec![vec!(1)]\n}\n",
    "pub fn f() { println!(\"\\0123\"); let _ = format!(\"\\012\"); }\n",
    "pub fn f() { std::println! { \"{}\", 1 }; vec! { 1, 2 }; }\n",
];

fn main() {
    let language = ryo_rust::linked();
    let engine = language.parse_engine();
    assert!(!language.walk_engine().includes_expansions(languages::Relation::SyntaxTree));
    assert!(!language.walk_engine().includes_expansions(languages::Relation::SyntaxChild));
    let relations = engine.file_relations().expect("the provider declares file relations");
    let visit: &dyn NumberedVisit = relations.numbered().expect("the provider takes the numbered way");
    let mut failures = String::new();
    for source in SOURCES {
        let bytes = source.as_bytes();
        let request = languages::ParseRequest {
            path: languages::SourcePath("src/lib.rs"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        };
        // The old table the linter read: the concrete prefix of the native table.
        let (mut direct, mut names) = (languages::Elements::default(), languages::Symbols::default());
        engine.build_native_file(request, &mut direct, &mut names, languages::NodeAccess::Query(&[])).expect("read");
        let old = languages::native_table(&direct).expect("a native table");
        let old_count = t::element_count(old).0;
        let mut reader = visit.open();
        let mut symbols = languages::Symbols::default();
        let mut found = None;
        reader
            .visit(request, &[], &[], None, &mut symbols, &mut |file, _| {
                let own = usize::from(t::own_end(file.tree));
                let same = own == old_count
                    && (0..own).all(|at| {
                        let (at, was) = (languages::Position(at as u32), languages::Position(at as u32));
                        t::kind_of(file.tree, at) == t::kind_of(old, was)
                            && t::parent_of(file.tree, at) == t::parent_of(old, was)
                            && t::span_of(file.tree, at) == t::span_of(old, was)
                    });
                found = Some((own, same, t::element_count(file.tree).0));
            })
            .expect("a Rust file is lent numbered");
        match found {
            Some((own, true, _)) if own == old_count => {}
            other => failures.push_str(&format!("{source:?}: old table {old_count} elements, lent tree {other:?}\n")),
        }
    }
    assert!(failures.is_empty(), "the lent tree's own part differs from the old table:\n{failures}");
}
