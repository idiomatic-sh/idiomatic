//! Proves the `DerivedTexts` value: a name and a file read as their text, any other value as none.

use super::value::DerivedTexts;
use crate::domain::table_texts::types::Texts;
use ryo_derived::domain::row_cells::types::ValueTexts;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, Integer, Offset, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::row_texts::types::CellText;

const SOURCE: &[u8] = b"main";

#[test]
fn a_name_reads_as_its_text() {
    let mut symbols = Symbols::default();
    let name = intern(&mut symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let texts = DerivedTexts(Texts {
        symbols: &symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
    });
    assert_eq!(texts.text(Value::Symbol(name)), Some(CellText("main")));
}

#[test]
fn a_file_reads_as_its_path() {
    let symbols = Symbols::default();
    let paths = [FilePath("src/lib.rs".into())];
    let texts = DerivedTexts(Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &paths,
        arenas: &[],
    });
    assert_eq!(
        texts.text(Value::File(FileId(0))),
        Some(CellText("src/lib.rs"))
    );
}

#[test]
fn a_number_has_no_text() {
    let symbols = Symbols::default();
    let texts = DerivedTexts(Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    });
    assert_eq!(texts.text(Value::Number(Integer(7))), None);
}
