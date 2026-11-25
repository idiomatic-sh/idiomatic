//! The `walk` contract's cases: the table the walk writes for a Rust file is the table the native reader writes for it today, element
//! for element (kind, parent, extent, field, decoded text), macro expansions included.

use ryo::languages::{
    self as l, ByteSlice, Count, FileFacts, FileId, FileVisit, NativeColumns, NodeAccess, NodeTable, RelationNeeds, SourcePath, Walk,
    WalkRequest, node_table as t,
};

/// What a table holds that a reader of it can tell apart.
#[derive(Debug, PartialEq, Eq)]
struct Shown {
    kinds: Vec<u16>,
    fields: Vec<u16>,
    parents: Vec<u32>,
    starts: Vec<u32>,
    ends: Vec<u32>,
    decoded: Vec<Option<String>>,
    own_end: usize,
    segments: Vec<(u32, u32, u32)>,
    errors: usize,
}

fn shown(table: &NodeTable) -> Shown {
    let count = usize::try_from(t::node_count(table).0).unwrap_or(0);
    Shown {
        kinds: t::kind_codes(table).to_vec(),
        fields: t::field_codes(table).to_vec(),
        parents: t::parent_ids(table).to_vec(),
        starts: t::starts(table).to_vec(),
        ends: t::ends(table).to_vec(),
        decoded: (0..count)
            .map(|at| t::decoded_of(table, l::Position(u32::try_from(at).unwrap_or(u32::MAX))).map(|text| text.0.to_owned()))
            .collect(),
        own_end: usize::from(t::own_end(table)),
        segments: t::embedded_segments(table).map(|each| (each.host.0, each.root.0, each.end.0)).collect(),
        errors: usize::try_from(t::syntax_error_count(table).0).unwrap_or(0),
    }
}

/// What the native reader writes for the bytes.
fn native(bytes: &[u8]) -> Shown {
    let mut table = NodeTable::default();
    let (mut origins, mut flags) = (Vec::new(), Vec::new());
    crate::outbound::syntax::adapters::ra_ap_syntax::native::read(
        bytes,
        &mut table,
        &mut origins,
        &mut flags,
        NativeColumns { fields: true, notes: false },
        NodeAccess::Inspect(&[]),
    );
    shown(&table)
}

/// Keeps what the walk lent.
struct Keep(Option<Shown>);

impl FileVisit for Keep {
    fn visit(&mut self, file: FileFacts<'_>) {
        self.0 = Some(shown(file.table));
    }
}

/// What the walk writes for the bytes.
fn walked<W: Walk>(walk: &W, bytes: &[u8]) -> Shown {
    let mut table = NodeTable::default();
    let mut builder = t::builder(&mut table, W::KIND_COUNT);
    let mut keep = Keep(None);
    let mut scratch = W::Scratch::default();
    walk.walk(
        WalkRequest { path: SourcePath("src/a.rs"), file: FileId(3), source: ByteSlice(bytes) },
        RelationNeeds(&[]),
        &mut builder,
        &mut scratch,
        &mut keep,
    )
    .expect("a Rust file is walked");
    keep.0.expect("the walk lent the file")
}

/// The first thing that differs between the two tables of one source, or nothing.
fn difference(bytes: &[u8]) -> String {
    let (old, new) = (native(bytes), walked(&crate::walk(), bytes));
    macro_rules! compare {
        ($($field:ident),*) => {$(
            if old.$field != new.$field {
                let at = old.$field.iter().zip(&new.$field).position(|(a, b)| a != b);
                return format!("{}: native {:?} walked {:?} (first at {at:?}; {} against {} entries)", stringify!($field),
                    old.$field.get(at.unwrap_or(0)), new.$field.get(at.unwrap_or(0)), old.$field.len(), new.$field.len());
            }
        )*};
    }
    compare!(kinds, fields, parents, starts, ends, decoded);
    if old.own_end != new.own_end || old.segments != new.segments || old.errors != new.errors {
        return format!("expansions: native {} {:?} {} walked {} {:?} {}", old.own_end, old.segments, old.errors, new.own_end, new.segments, new.errors);
    }
    String::new()
}

/// Sources that cover the grammar's corners: macro calls whose arguments are read again, nested ones, decoded strings, numbers,
/// attributes, errors and bytes that are no text.
const SOURCES: &[&[u8]] = &[
    b"fn main() {}\n",
    br#"#[path = "a.rs"] mod a; fn f(){ println!("{}", vec![1u8, 2]); let s = "a\nb"; }"#,
    b"fn g(x: &str) { /* \xff */ let y = (",
    b"fn h() { assert!(matches!(Some(1), Some(n) if n > 0)); let v = vec![0; 3]; format!(\"{x}\", x = vec![vec![1]]); }",
    b"/// doc\n#[derive(Debug)]\npub struct S<T: Copy = u8> where T: Clone { pub a: T, b: [u8; 4] }\nimpl S<u8> { fn f(&self) -> 0x1f_u8 { 1.5e3 as u8 } }\n",
    b"use std::{io, fmt::Write as _};\nmacro_rules! m { ($x:expr) => { $x + 1 }; }\nfn k() { let _ = m!(1); r#\"raw\"#; b\"bytes\"; 'c'; }\n",
    b"",
];

#[test]
fn the_walk_writes_the_table_the_native_reader_writes() {
    for source in SOURCES {
        assert_eq!(difference(source), "", "source {:?}", String::from_utf8_lossy(source));
    }
}

#[test]
fn the_kind_count_is_the_count_of_the_language_s_kinds() {
    fn count<W: Walk>(_: &W) -> Count {
        W::KIND_COUNT
    }
    assert_eq!(count(&crate::walk()).0, crate::domain::syntax::service::kinds().count());
}
