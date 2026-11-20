/// Declares how a language fills a relation for the engine, from its per-file view: the one way an item implements
/// `ryo::languages::FillItem`, and, with a key clause, `ryo::languages::KeyedFillItem`.
///
/// ```text
/// ryo::fills! {
///     Item => languages::Relation, walk(Language) |file, words, rows| {
///         // `file`: &mut the language's `Fillers::File` (its parse in place, a typed node per id, its model on first need);
///         // `words`: the file's words (`facts::intern` a built text, `facts::fixed` a fixed name's word by its handle);
///         // `rows`: the relation's rows, `rows.write(Row { .. })`, `rows.partial(..)` for a part it cannot read.
///     }
///     key(languages::Column::Relation(languages::RelationColumn::Id)) |file, words, key, rows| {
///         // `key`: the cell the column holds in the rows wanted (a node's id, a number, or a word read with `facts::text`).
///         // Writes the rows whose column holds `key`, and no other row.
///     }
/// }
/// ```
///
/// The first arm writes every row of the relation for the file. A key clause writes the rows of one key of the column it names;
/// an item has one for each column the relation's catalog item declares among its `keys`, and may have more. The engine asks
/// for the whole relation or for one key, and a fill appends to what earlier fills of the file wrote.
///
/// The language's `Fillers` function of the relation runs the item: `fn syntax_error(file, words, wanted, rows) -> Fill {
/// fillers::filled::<SyntaxErrors>(file, words, wanted, rows) }` for an item with no key clause, which answers a key with every
/// row and says so, and `fillers::filled_keyed::<TokenLinks>(file, words, wanted, rows)` for an item with one, as the language
/// crate's fixture language does (`fillers::fixtures::Walking`).
#[macro_export]
macro_rules! fills {
    (
        $item:ident => $relation:ty, walk ($walk:ty)
        |$file:ident, $words:ident, $rows:ident|
        { $($body:tt)* }
        $(
            key ($column:expr)
            |$key_file:ident, $key_words:ident, $key:ident, $key_rows:ident|
            { $($key_body:tt)* }
        )*
    ) => {
        impl $crate::languages::FillItem for $item {
            type Walk = $walk;
            type Relation = $relation;

            fn fill(
                $file: &mut <$walk as $crate::languages::Fillers>::File<'_>,
                $words: &mut $crate::languages::FileWords,
                $rows: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                $($body)*
            }
        }

        $crate::fills! {
            @keyed $item, $relation, $walk;
            $( ($column) |$key_file, $key_words, $key, $key_rows| { $($key_body)* } )*
        }
    };
    (@keyed $item:ident, $relation:ty, $walk:ty;) => {};
    (
        @keyed $item:ident, $relation:ty, $walk:ty;
        $( ($column:expr) |$file:ident, $words:ident, $key:ident, $rows:ident| { $($body:tt)* } )+
    ) => {
        impl $crate::languages::KeyedFillItem for $item {
            const KEYS: &'static [$crate::languages::Column] = &[ $( $column ),+ ];

            fn fill_key(
                file: &mut <$walk as $crate::languages::Fillers>::File<'_>,
                words: &mut $crate::languages::FileWords,
                column: $crate::languages::Column,
                key: $crate::languages::CellWord,
                rows: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                $(
                    if column == $column {
                        let ($file, $words, $key, $rows) = (&mut *file, &mut *words, key, &mut *rows);
                        $($body)*
                        return;
                    }
                )+
            }
        }
    };
}
